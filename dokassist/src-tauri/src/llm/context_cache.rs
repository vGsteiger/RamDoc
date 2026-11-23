use serde::{Deserialize, Serialize};

/// Bump whenever the agent system/tool prompt semantics change.
pub const AGENT_PROMPT_VERSION: &str = "agent-v3";

/// What a context is kept for, which decides eviction order: one-off
/// generations (reports, summaries, metadata extraction) are evicted before
/// agent conversations, whose next turn would otherwise prefill from zero.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// A multi-turn agent conversation whose prefix the next turn reuses.
    #[default]
    Conversation,
    /// A single generation keyed by its prompt digest (`raw:`/`think:`).
    OneOff,
}

/// The caller-owned portion of an inference-context identity.
///
/// Patient identity and revision are deliberately separate fields: a context
/// can never be reused for another patient, and updating a patient invalidates
/// every context built from the previous record revision.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InferenceSession {
    pub conversation_id: String,
    pub patient_id: Option<String>,
    pub patient_revision: Option<String>,
    pub prompt_version: String,
    pub adapter_hash: String,
    pub kind: SessionKind,
}

impl InferenceSession {
    pub fn agent(
        conversation_id: impl Into<String>,
        patient_id: Option<String>,
        patient_revision: Option<String>,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            patient_id,
            patient_revision,
            prompt_version: AGENT_PROMPT_VERSION.to_string(),
            adapter_hash: "none".to_string(),
            kind: SessionKind::Conversation,
        }
    }

    pub fn isolated(id: impl Into<String>) -> Self {
        Self {
            conversation_id: id.into(),
            patient_id: None,
            patient_revision: None,
            prompt_version: "raw-v1".to_string(),
            adapter_hash: "none".to_string(),
            kind: SessionKind::OneOff,
        }
    }
}

/// Complete identity for a KV context. Every input that can alter tokenization
/// or KV layout is represented, in addition to patient/conversation isolation.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct ContextKey {
    pub model_hash: String,
    pub chat_template_hash: String,
    pub prompt_version: String,
    pub adapter_hash: String,
    pub context_size: usize,
    pub batch_size: u32,
    pub kv_config_hash: String,
    pub conversation_id: String,
    pub patient_id: Option<String>,
    pub patient_revision: Option<String>,
    pub kind: SessionKind,
}

impl ContextKey {
    /// Whether two keys belong to the same logical conversation. Used to
    /// eagerly invalidate stale revisions/configurations instead of waiting
    /// for LRU eviction.
    pub fn same_logical_context(&self, other: &Self) -> bool {
        self.conversation_id == other.conversation_id && self.patient_id == other.patient_id
    }

    /// Whether a context allocated for `self` has the KV layout `other`
    /// needs, so it can be re-keyed instead of freed and reallocated.
    pub fn same_runtime_profile(&self, other: &Self) -> bool {
        self.model_hash == other.model_hash
            && self.chat_template_hash == other.chat_template_hash
            && self.adapter_hash == other.adapter_hash
            && self.context_size == other.context_size
            && self.batch_size == other.batch_size
            && self.kv_config_hash == other.kv_config_hash
    }

    /// Whether a re-keyed context may keep a token prefix cached for `self`:
    /// only for the same patient record revision. Otherwise its KV is cleared
    /// in full, so no patient's context survives a switch to another.
    pub fn may_share_prefix(&self, other: &Self) -> bool {
        self.patient_id == other.patient_id && self.patient_revision == other.patient_revision
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextCacheTelemetry {
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub evictions: u64,
    pub reused_tokens: u64,
    pub evaluated_tokens: u64,
    pub estimated_prefill_saved_ms: f64,
    pub resident_contexts: usize,
    pub max_contexts: usize,
    /// Hybrid/recurrent models only: prefix reuse via a restored recurrent
    /// state snapshot, where plain KV rollback is impossible.
    #[serde(default)]
    pub checkpoint_restores: u64,
    #[serde(default)]
    pub checkpoints_created: u64,
    /// `LlamaContext` allocations (KV cache plus compute buffers).
    #[serde(default)]
    pub contexts_created: u64,
    /// Key misses served by re-keying a resident context with the same
    /// runtime profile instead of freeing it and allocating a new one.
    #[serde(default)]
    pub context_allocations_reused: u64,
}

/// How to serve a key from the resident contexts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SlotAction {
    /// The key is resident at this index.
    Hit(usize),
    /// Re-key the context at this index for the new key, keeping its
    /// allocation. Without `keep_prefix` its KV must be cleared in full.
    Reuse { index: usize, keep_prefix: bool },
    /// Allocate a new context.
    Create,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotPlan {
    pub action: SlotAction,
    /// Indices to drop before acting, ascending. Never the `Hit`/`Reuse` index.
    pub evict: Vec<usize>,
}

/// Decide which resident context serves `key`, given each resident key and
/// its last use. Stale entries (same conversation and patient, different key)
/// always leave. When the pool is full, entries leave in this order: stale,
/// one-off, conversation, least recently used first within each group. The
/// first leaving entry with the runtime profile `key` needs is re-keyed
/// instead of freed, so a context is only allocated when the pool has room
/// or no leaving entry has that profile.
pub(crate) fn plan_slot(
    resident: &[(&ContextKey, u64)],
    key: &ContextKey,
    capacity: usize,
) -> SlotPlan {
    let stale = |candidate: &ContextKey| candidate.same_logical_context(key) && candidate != key;
    if let Some(index) = resident.iter().position(|(candidate, _)| *candidate == key) {
        let evict = (0..resident.len())
            .filter(|&i| stale(resident[i].0))
            .collect();
        return SlotPlan {
            action: SlotAction::Hit(index),
            evict,
        };
    }

    let class = |candidate: &ContextKey| match (stale(candidate), candidate.kind) {
        (true, _) => 0u8,
        (false, SessionKind::OneOff) => 1,
        (false, SessionKind::Conversation) => 2,
    };
    let mut order: Vec<usize> = (0..resident.len()).collect();
    order.sort_by_key(|&i| (class(resident[i].0), resident[i].1));
    let stale_count = resident
        .iter()
        .filter(|(candidate, _)| stale(candidate))
        .count();
    let must_free = (resident.len() + 1).saturating_sub(capacity.max(1));
    let mut leaving: Vec<usize> = order[..must_free.max(stale_count)].to_vec();

    let action = match leaving
        .iter()
        .position(|&i| resident[i].0.same_runtime_profile(key))
    {
        Some(position) => {
            let index = leaving.remove(position);
            SlotAction::Reuse {
                index,
                keep_prefix: resident[index].0.may_share_prefix(key),
            }
        }
        None => SlotAction::Create,
    };
    leaving.sort_unstable();
    SlotPlan {
        action,
        evict: leaving,
    }
}

/// Newest checkpoint position usable for a shared prefix of `reused` tokens:
/// it must not extend past the shared prefix and must cover at least one
/// token (position 0 is a full reset, not a reuse).
pub(crate) fn checkpoint_for_prefix(positions: &[usize], reused: usize) -> Option<usize> {
    positions
        .iter()
        .copied()
        .filter(|&n| n > 0 && n <= reused)
        .max()
}

/// Where to snapshot a prompt of `prompt_len` tokens whose first `reused`
/// tokens are already in the context: up to `tail` tokens before the end (at
/// most half of the uncached remainder), so the next turn (this prompt plus
/// an answer) can resume almost all of it. `None` when fewer than two tokens
/// remain to be evaluated.
pub(crate) fn checkpoint_position(prompt_len: usize, reused: usize, tail: usize) -> Option<usize> {
    let remaining = prompt_len.checked_sub(reused)?;
    if remaining < 2 {
        return None;
    }
    let tail = tail.min(remaining / 2).max(1);
    Some(prompt_len - tail)
}

pub(crate) fn reusable_prefix<T: PartialEq>(cached: &[T], requested: &[T]) -> usize {
    cached
        .iter()
        .zip(requested)
        .take_while(|(left, right)| left == right)
        .count()
        // Sampling reads the logits from the last decoded token. Re-evaluate
        // that token so rollback never samples stale logits from a completion.
        .saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(patient: &str, revision: &str, model: &str) -> ContextKey {
        ContextKey {
            model_hash: model.into(),
            chat_template_hash: "template".into(),
            prompt_version: AGENT_PROMPT_VERSION.into(),
            adapter_hash: "none".into(),
            context_size: 16_384,
            batch_size: 2_048,
            kv_config_hash: "f16:512:enabled".into(),
            conversation_id: "session-1".into(),
            patient_id: Some(patient.into()),
            patient_revision: Some(revision.into()),
            kind: SessionKind::Conversation,
        }
    }

    #[test]
    fn checkpoint_selection_prefers_the_newest_within_the_shared_prefix() {
        assert_eq!(checkpoint_for_prefix(&[100, 900], 950), Some(900));
        assert_eq!(checkpoint_for_prefix(&[100, 900], 899), Some(100));
        assert_eq!(checkpoint_for_prefix(&[100, 900], 99), None);
        assert_eq!(checkpoint_for_prefix(&[0], 50), None);
        assert_eq!(checkpoint_for_prefix(&[], 50), None);
    }

    #[test]
    fn checkpoint_lands_near_the_prompt_end() {
        assert_eq!(checkpoint_position(1_000, 0, 64), Some(936));
        // Short or mostly cached prompts snapshot halfway through the remainder.
        assert_eq!(checkpoint_position(1_000, 936, 64), Some(968));
        assert_eq!(checkpoint_position(30, 0, 64), Some(15));
        assert_eq!(checkpoint_position(30, 29, 64), None);
    }

    #[test]
    fn cache_keys_isolate_patient_revision_model_and_template_inputs() {
        let base = key("patient-a", "revision-1", "model-a");
        assert_ne!(base, key("patient-b", "revision-1", "model-a"));
        assert_ne!(base, key("patient-a", "revision-2", "model-a"));
        assert_ne!(base, key("patient-a", "revision-1", "model-b"));

        let mut template_changed = base.clone();
        template_changed.chat_template_hash = "other-template".into();
        assert_ne!(base, template_changed);

        let mut prompt_changed = base.clone();
        prompt_changed.prompt_version = format!("{AGENT_PROMPT_VERSION}-next");
        assert_ne!(base, prompt_changed);

        let mut adapter_changed = base.clone();
        adapter_changed.adapter_hash = "adapter-a".into();
        assert_ne!(base, adapter_changed);

        let mut kv_changed = base.clone();
        kv_changed.kv_config_hash = "q8:512:enabled".into();
        assert_ne!(base, kv_changed);
    }

    #[test]
    fn revision_change_is_the_same_logical_context_but_a_different_key() {
        let old = key("patient-a", "revision-1", "model-a");
        let new = key("patient-a", "revision-2", "model-a");
        assert!(old.same_logical_context(&new));
        assert_ne!(old, new);
    }

    #[test]
    fn prefix_reuse_refreshes_last_token_logits() {
        assert_eq!(reusable_prefix(&[1, 2, 3, 8], &[1, 2, 3, 9]), 2);
        assert_eq!(reusable_prefix(&[1, 2, 3], &[1, 2, 3]), 2);
        assert_eq!(reusable_prefix(&[1], &[1]), 0);
        assert_eq!(reusable_prefix(&[1, 2], &[9, 2]), 0);
    }

    fn conversation(id: &str, patient: &str) -> ContextKey {
        let mut key = key(patient, "revision-1", "model-a");
        key.conversation_id = id.into();
        key
    }

    fn one_off(digest: &str) -> ContextKey {
        let mut key = key("unused", "unused", "model-a");
        key.conversation_id = format!("raw:{digest}");
        key.prompt_version = "raw-v1".into();
        key.patient_id = None;
        key.patient_revision = None;
        key.kind = SessionKind::OneOff;
        key
    }

    #[test]
    fn session_constructors_set_the_kind() {
        assert_eq!(
            InferenceSession::agent("chat", None, None).kind,
            SessionKind::Conversation
        );
        assert_eq!(
            InferenceSession::isolated("raw:abc").kind,
            SessionKind::OneOff
        );
    }

    #[test]
    fn resident_key_is_a_hit() {
        let chat = conversation("chat", "patient-a");
        let plan = plan_slot(&[(&chat, 1)], &chat, 1);
        assert_eq!(plan.action, SlotAction::Hit(0));
        assert!(plan.evict.is_empty());
    }

    #[test]
    fn miss_with_unchanged_profile_reuses_the_allocation() {
        let chat = conversation("chat", "patient-a");
        let plan = plan_slot(&[(&chat, 1)], &one_off("report"), 1);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 0,
                keep_prefix: false
            }
        );
        assert!(plan.evict.is_empty());
    }

    #[test]
    fn miss_with_changed_profile_evicts_and_creates() {
        let chat = conversation("chat", "patient-a");
        let mut kv_changed = conversation("chat-2", "patient-a");
        kv_changed.kv_config_hash = "q8:512:enabled".into();
        let plan = plan_slot(&[(&chat, 1)], &kv_changed, 1);
        assert_eq!(plan.action, SlotAction::Create);
        assert_eq!(plan.evict, vec![0]);

        let mut model_changed = conversation("chat-2", "patient-a");
        model_changed.model_hash = "model-b".into();
        assert_eq!(
            plan_slot(&[(&chat, 1)], &model_changed, 1).action,
            SlotAction::Create
        );

        let mut context_changed = conversation("chat-2", "patient-a");
        context_changed.context_size = 32_768;
        assert_eq!(
            plan_slot(&[(&chat, 1)], &context_changed, 1).action,
            SlotAction::Create
        );
    }

    #[test]
    fn pool_with_room_creates_without_evicting() {
        let chat = conversation("chat", "patient-a");
        let plan = plan_slot(&[(&chat, 1)], &one_off("report"), 2);
        assert_eq!(plan.action, SlotAction::Create);
        assert!(plan.evict.is_empty());
        assert_eq!(
            plan_slot(&[], &chat, 1),
            SlotPlan {
                action: SlotAction::Create,
                evict: vec![]
            }
        );
    }

    #[test]
    fn one_off_displaces_another_one_off_before_a_conversation() {
        let chat = conversation("chat", "patient-a");
        let old_report = one_off("old");
        // The one-off was used more recently, yet it leaves first.
        let resident = [(&chat, 1), (&old_report, 5)];
        let plan = plan_slot(&resident, &one_off("new"), 2);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: true
            }
        );
        assert!(plan.evict.is_empty());

        // A new conversation also takes the one-off slot, not the other chat.
        let plan = plan_slot(&resident, &conversation("chat-2", "patient-b"), 2);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: false
            }
        );
    }

    #[test]
    fn conversations_alone_are_evicted_least_recently_used_first() {
        let older = conversation("older", "patient-a");
        let newer = conversation("newer", "patient-b");
        let plan = plan_slot(&[(&newer, 9), (&older, 3)], &one_off("report"), 2);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: false
            }
        );
    }

    #[test]
    fn patient_change_clears_the_kv_and_same_record_keeps_the_prefix() {
        let chat = conversation("chat", "patient-a");
        assert_eq!(
            plan_slot(&[(&chat, 1)], &conversation("chat-2", "patient-b"), 1).action,
            SlotAction::Reuse {
                index: 0,
                keep_prefix: false
            }
        );
        assert_eq!(
            plan_slot(&[(&chat, 1)], &conversation("chat-2", "patient-a"), 1).action,
            SlotAction::Reuse {
                index: 0,
                keep_prefix: true
            }
        );

        // A new record revision of the same conversation makes the resident
        // entry stale: its allocation is reused, but none of its tokens.
        let revised = key("patient-a", "revision-2", "model-a");
        let stale = key("patient-a", "revision-1", "model-a");
        assert_eq!(
            plan_slot(&[(&stale, 1)], &revised, 2),
            SlotPlan {
                action: SlotAction::Reuse {
                    index: 0,
                    keep_prefix: false
                },
                evict: vec![]
            }
        );
    }

    #[test]
    fn stale_entries_always_leave() {
        let current = key("patient-a", "revision-2", "model-a");
        let stale = key("patient-a", "revision-1", "model-a");
        let report = one_off("report");

        // On a hit, a stale twin is dropped.
        let plan = plan_slot(&[(&stale, 1), (&current, 2)], &current, 2);
        assert_eq!(plan.action, SlotAction::Hit(1));
        assert_eq!(plan.evict, vec![0]);

        // With room to spare, the stale entry is re-keyed rather than kept.
        let plan = plan_slot(&[(&report, 1), (&stale, 2)], &current, 3);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: false
            }
        );
        assert!(plan.evict.is_empty());

        // A stale entry with another profile is freed, not reused.
        let mut stale_q8 = stale.clone();
        stale_q8.kv_config_hash = "q8:512:enabled".into();
        let plan = plan_slot(&[(&report, 1), (&stale_q8, 2)], &current, 3);
        assert_eq!(plan.action, SlotAction::Create);
        assert_eq!(plan.evict, vec![1]);
    }

    #[test]
    fn a_shrunken_pool_evicts_down_to_capacity() {
        let a = conversation("a", "patient-a");
        let b = conversation("b", "patient-b");
        let c = one_off("c");
        let plan = plan_slot(&[(&a, 1), (&b, 2), (&c, 3)], &one_off("d"), 1);
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 2,
                keep_prefix: true
            }
        );
        assert_eq!(plan.evict, vec![0, 1]);
    }
}

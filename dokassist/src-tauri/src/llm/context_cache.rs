use serde::{Deserialize, Serialize};

/// Bump whenever the agent system/tool prompt semantics change.
pub const AGENT_PROMPT_VERSION: &str = "agent-v3";

/// Prompt version of task-keyed one-off sessions ([`InferenceSession::one_off`]).
pub const ONE_OFF_PROMPT_VERSION: &str = "task-v1";

/// What a context is kept for, which decides eviction order: one-off
/// generations (reports, summaries, metadata extraction) are evicted before
/// agent conversations, whose next turn would otherwise prefill from zero.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// A multi-turn agent conversation whose prefix the next turn reuses.
    #[default]
    Conversation,
    /// A single generation, keyed by task family and patient
    /// ([`InferenceSession::one_off`]) or by its prompt digest (`raw:`/`think:`).
    OneOff,
}

/// Task family of a one-off generation. Consecutive generations of one
/// family for one patient record share a key, so the next one finds the
/// previous context resident and reuses their common token prefix.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum OneOffTask {
    Report,
    Letter,
    Summary,
    Evidence,
    Improve,
    Extract,
    /// Agent history condensed when a conversation nears the context limit.
    HistorySummary,
    /// Patient context and notes condensed before a report, letter or summary.
    Condense,
}

impl OneOffTask {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Letter => "letter",
            Self::Summary => "summary",
            Self::Evidence => "evidence",
            Self::Improve => "improve",
            Self::Extract => "extract",
            Self::HistorySummary => "history-summary",
            Self::Condense => "condense",
        }
    }
}

/// A patient record at one revision (`evidence::provenance::patient_revision`,
/// the revision agent conversations are keyed by too). Identity and
/// revision travel together, so a patient-keyed context cannot outlive an
/// edit to the record.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PatientRevision {
    pub patient_id: String,
    pub revision: String,
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
    /// The patient-free static part this session's prompts start with, if
    /// the caller declared one. It does not change the context key; the
    /// engine uses it to seed an empty context from a snapshot.
    pub static_prefix: Option<StaticPrefix>,
}

/// Prompt version of [`StaticPrefix`]es built from the fixed one-off system
/// prompts (`SYSTEM_PROMPT_DE`/`FR`).
pub const SYSTEM_PROMPT_VERSION: &str = "system-v1";

/// Text that starts patient-specific prompt content. A static prefix must
/// never contain it; the engine refuses to snapshot one that does.
pub(crate) const PATIENT_SECTION_MARKERS: &[&str] = &[
    "Du arbeitest im Kontext von Patient-ID",
    "Aktuelle Patientendaten",
];

/// Whether `text` contains the start of a patient section.
pub(crate) fn contains_patient_marker(text: &str) -> bool {
    PATIENT_SECTION_MARKERS
        .iter()
        .any(|marker| text.contains(marker))
}

/// One patient-free chat used to locate a static prefix.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PrefixProbe {
    pub system: String,
    pub user: String,
}

/// The static part of a family of prompts (agent persona, tools and
/// tool-call format for one scope, or a fixed one-off system prompt).
///
/// It is declared by two probe chats that are identical up to the end of
/// the static part and differ right after it, and contain no patient data.
/// The engine formats both with the model's chat template, tokenizes them
/// and takes their common token prefix ([`static_prefix_len`]), so the
/// boundary follows the template without parsing it.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StaticPrefix {
    /// Scope or task family, e.g. `agent:patient` or `system:de`.
    pub family: String,
    pub prompt_version: String,
    pub probes: [PrefixProbe; 2],
}

impl InferenceSession {
    /// Declare the static prefix this session's prompts start with.
    pub fn with_static_prefix(mut self, static_prefix: Option<StaticPrefix>) -> Self {
        if static_prefix.is_some() {
            self.static_prefix = static_prefix;
        }
        self
    }

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
            static_prefix: None,
        }
    }

    /// A one-off generation keyed by task family and, when it reads a patient
    /// record, by that patient and record revision. The same task for two
    /// patients, or for two revisions of one record, never shares a key;
    /// `reusable_prefix` still limits reuse within a key to identical tokens.
    pub fn one_off(task: OneOffTask, patient: Option<&PatientRevision>) -> Self {
        Self {
            conversation_id: format!("task:{}", task.as_str()),
            patient_id: patient.map(|patient| patient.patient_id.clone()),
            patient_revision: patient.map(|patient| patient.revision.clone()),
            prompt_version: ONE_OFF_PROMPT_VERSION.to_string(),
            adapter_hash: "none".to_string(),
            kind: SessionKind::OneOff,
            static_prefix: None,
        }
    }

    /// The patient record this session is bound to, when it has both an id
    /// and a revision.
    pub fn patient(&self) -> Option<PatientRevision> {
        Some(PatientRevision {
            patient_id: self.patient_id.clone()?,
            revision: self.patient_revision.clone()?,
        })
    }

    /// A one-off keyed by the prompt digest: only an exact repeat of the
    /// prompt finds it resident. The fallback when no task session is given.
    pub fn isolated(id: impl Into<String>) -> Self {
        Self {
            conversation_id: id.into(),
            patient_id: None,
            patient_revision: None,
            prompt_version: "raw-v1".to_string(),
            adapter_hash: "none".to_string(),
            kind: SessionKind::OneOff,
            static_prefix: None,
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
    /// Contexts seeded from a static-prefix snapshot instead of prefilling
    /// the system/tool prefix again (see [`PrefixSnapshotStore`]).
    #[serde(default)]
    pub prefix_snapshot_restores: u64,
    #[serde(default)]
    pub prefix_snapshots_created: u64,
    /// Bytes currently held by static-prefix snapshots, in memory only.
    #[serde(default)]
    pub prefix_snapshot_bytes: u64,
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

/// Segment boundaries for prefilling `tokens[reused..len]`, split at each
/// cut strictly inside that range (checkpoint or snapshot positions), so
/// state can be captured between segments. Ascending, starting at `reused`
/// and ending at `len`.
pub(crate) fn prefill_bounds(reused: usize, len: usize, cuts: &[Option<usize>]) -> Vec<usize> {
    let mut bounds = vec![reused];
    bounds.extend(
        cuts.iter()
            .flatten()
            .copied()
            .filter(|&at| at > reused && at < len),
    );
    bounds.push(len.max(reused));
    bounds.sort_unstable();
    bounds.dedup();
    bounds
}

/// Length of the static prefix shared by two probe prompts: their common
/// token prefix minus one. The last common token may still merge with what
/// follows it in a real prompt, and a restored prefix must leave at least
/// one prompt token to decode so sampling reads fresh logits.
pub(crate) fn static_prefix_len<T: PartialEq>(left: &[T], right: &[T]) -> usize {
    reusable_prefix(left, right)
}

/// Contexts with this key can restore each other's static-prefix snapshots:
/// every input that changes the tokens or the KV layout of the prefix, and
/// no patient or conversation identity (the prefix holds no patient data).
/// llama.cpp rejects a state restored into another `n_ctx` or KV type with a
/// size mismatch, so the runtime profile is part of the key.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct PrefixSnapshotKey {
    pub model_hash: String,
    pub chat_template_hash: String,
    pub prompt_version: String,
    pub adapter_hash: String,
    pub context_size: usize,
    pub batch_size: u32,
    pub kv_config_hash: String,
    pub family: String,
}

impl PrefixSnapshotKey {
    pub fn new(context: &ContextKey, prefix: &StaticPrefix) -> Self {
        Self {
            model_hash: context.model_hash.clone(),
            chat_template_hash: context.chat_template_hash.clone(),
            prompt_version: prefix.prompt_version.clone(),
            adapter_hash: context.adapter_hash.clone(),
            context_size: context.context_size,
            batch_size: context.batch_size,
            kv_config_hash: context.kv_config_hash.clone(),
            family: prefix.family.clone(),
        }
    }

    /// Whether a context keyed `context` has the runtime profile this
    /// snapshot was captured with.
    pub fn same_runtime_profile(&self, context: &ContextKey) -> bool {
        self.model_hash == context.model_hash
            && self.chat_template_hash == context.chat_template_hash
            && self.adapter_hash == context.adapter_hash
            && self.context_size == context.context_size
            && self.batch_size == context.batch_size
            && self.kv_config_hash == context.kv_config_hash
    }
}

/// Snapshots kept per key. Two, because families whose chat header depends
/// on the reasoning effort (Gemma 4, Harmony) start the probe and the answer
/// with different prefixes.
pub(crate) const MAX_PREFIX_SNAPSHOTS_PER_KEY: usize = 2;
/// Snapshots kept in total, across scopes and task families.
pub(crate) const MAX_PREFIX_SNAPSHOTS: usize = 4;

pub(crate) struct PrefixSnapshot<T, S> {
    pub key: PrefixSnapshotKey,
    /// The prefix tokens; the state is sequence 0 right after them.
    pub tokens: Vec<T>,
    pub state: S,
    pub bytes: usize,
    last_used: u64,
}

/// In-memory store of static-prefix states (never written to disk). It is
/// owned by the context pool, so it is dropped with the resident contexts
/// (model unload, app lock, reset).
pub(crate) struct PrefixSnapshotStore<T, S> {
    entries: Vec<PrefixSnapshot<T, S>>,
    clock: u64,
}

impl<T: PartialEq, S> Default for PrefixSnapshotStore<T, S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            clock: 0,
        }
    }
}

impl<T: PartialEq, S> PrefixSnapshotStore<T, S> {
    /// The longest snapshot for `key` whose tokens are a strict prefix of
    /// `prompt` (at least one prompt token must remain to be decoded).
    pub fn find(&mut self, key: &PrefixSnapshotKey, prompt: &[T]) -> Option<usize> {
        let index = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.key == *key
                    && !entry.tokens.is_empty()
                    && entry.tokens.len() < prompt.len()
                    && prompt.starts_with(&entry.tokens)
            })
            .max_by_key(|(_, entry)| entry.tokens.len())
            .map(|(index, _)| index)?;
        self.clock += 1;
        self.entries[index].last_used = self.clock;
        Some(index)
    }

    pub fn get(&self, index: usize) -> &PrefixSnapshot<T, S> {
        &self.entries[index]
    }

    pub fn contains(&self, key: &PrefixSnapshotKey, tokens: &[T]) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.key == *key && entry.tokens == tokens)
    }

    pub fn remove(&mut self, index: usize) {
        self.entries.remove(index);
    }

    /// Store a snapshot, replacing one with the same key and tokens and
    /// evicting least recently used ones to stay within the per-key and
    /// total caps and `budget_bytes`. Returns `false`, storing nothing, when
    /// the snapshot alone exceeds the budget.
    pub fn insert(
        &mut self,
        key: PrefixSnapshotKey,
        tokens: Vec<T>,
        state: S,
        bytes: usize,
        budget_bytes: usize,
    ) -> bool {
        self.entries
            .retain(|entry| !(entry.key == key && entry.tokens == tokens));
        if bytes > budget_bytes {
            return false;
        }
        while self.entries.iter().filter(|entry| entry.key == key).count()
            >= MAX_PREFIX_SNAPSHOTS_PER_KEY
        {
            self.evict_lru(Some(&key));
        }
        while !self.entries.is_empty()
            && (self.entries.len() >= MAX_PREFIX_SNAPSHOTS
                || self.bytes().saturating_add(bytes) > budget_bytes)
        {
            self.evict_lru(None);
        }
        self.clock += 1;
        self.entries.push(PrefixSnapshot {
            key,
            tokens,
            state,
            bytes,
            last_used: self.clock,
        });
        true
    }

    fn evict_lru(&mut self, key: Option<&PrefixSnapshotKey>) {
        if let Some((index, _)) = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| key.is_none_or(|key| entry.key == *key))
            .min_by_key(|(_, entry)| entry.last_used)
        {
            self.entries.remove(index);
        }
    }

    /// Drop every snapshot `keep` rejects (e.g. another runtime profile).
    pub fn retain(&mut self, keep: impl Fn(&PrefixSnapshotKey) -> bool) {
        self.entries.retain(|entry| keep(&entry.key));
    }

    pub fn bytes(&self) -> usize {
        self.entries.iter().map(|entry| entry.bytes).sum()
    }
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

    fn record(patient: &str, revision: &str) -> PatientRevision {
        PatientRevision {
            patient_id: patient.into(),
            revision: revision.into(),
        }
    }

    /// The context key the engine derives from `session` (see
    /// `LlmEngine::context_key`), with a fixed runtime profile.
    fn session_key(session: &InferenceSession) -> ContextKey {
        let mut key = key("unused", "unused", "model-a");
        key.prompt_version = session.prompt_version.clone();
        key.adapter_hash = session.adapter_hash.clone();
        key.conversation_id = session.conversation_id.clone();
        key.patient_id = session.patient_id.clone();
        key.patient_revision = session.patient_revision.clone();
        key.kind = session.kind;
        key
    }

    fn task_key(task: OneOffTask, patient: Option<(&str, &str)>) -> ContextKey {
        let patient = patient.map(|(id, revision)| record(id, revision));
        session_key(&InferenceSession::one_off(task, patient.as_ref()))
    }

    const TASKS: [OneOffTask; 8] = [
        OneOffTask::Report,
        OneOffTask::Letter,
        OneOffTask::Summary,
        OneOffTask::Evidence,
        OneOffTask::Improve,
        OneOffTask::Extract,
        OneOffTask::HistorySummary,
        OneOffTask::Condense,
    ];

    #[test]
    fn one_off_sessions_are_keyed_by_task_and_patient_record() {
        let patient = record("patient-a", "p1:abc");
        for task in TASKS {
            let session = InferenceSession::one_off(task, Some(&patient));
            assert_eq!(session.conversation_id, format!("task:{}", task.as_str()));
            assert_eq!(session.patient_id.as_deref(), Some("patient-a"));
            assert_eq!(session.patient_revision.as_deref(), Some("p1:abc"));
            assert_eq!(session.prompt_version, ONE_OFF_PROMPT_VERSION);
            assert_eq!(session.kind, SessionKind::OneOff);
            assert_eq!(session.patient(), Some(patient.clone()));

            let unbound = InferenceSession::one_off(task, None);
            assert_eq!(unbound.patient_id, None);
            assert_eq!(unbound.patient_revision, None);
            assert_eq!(unbound.patient(), None);
        }
        let mut ids: Vec<_> = TASKS.iter().map(|task| task.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TASKS.len(), "task ids must be distinct");

        // Task ids never collide with digest-keyed or agent sessions.
        assert!(!InferenceSession::isolated("raw:abc")
            .conversation_id
            .starts_with("task:"));
        assert_eq!(InferenceSession::agent("chat", None, None).patient(), None);
    }

    #[test]
    fn the_same_task_for_two_patients_never_shares_a_key() {
        for task in TASKS {
            let a = task_key(task, Some(("patient-a", "p1:same")));
            let b = task_key(task, Some(("patient-b", "p1:same")));
            assert_ne!(a, b);
            assert_ne!(a, task_key(task, None));
            // Two patients are never the same logical context, so neither
            // ever invalidates or re-keys the other as "stale".
            assert!(!a.same_logical_context(&b));
            assert!(!a.may_share_prefix(&b));
        }
    }

    #[test]
    fn a_new_record_revision_is_a_new_key_for_the_same_task() {
        let old = task_key(OneOffTask::Report, Some(("patient-a", "p1:old")));
        let new = task_key(OneOffTask::Report, Some(("patient-a", "p1:new")));
        assert_ne!(old, new);
        assert!(old.same_logical_context(&new));
        // The outdated context is re-keyed with its KV cleared in full.
        assert_eq!(
            plan_slot(&[(&old, 1)], &new, 2),
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
    fn report_regeneration_for_the_same_patient_hits_the_resident_context() {
        let report = task_key(OneOffTask::Report, Some(("patient-a", "p1:abc")));
        let chat = conversation("chat", "patient-a");
        // Different notes or instructions do not change the key, so the
        // regeneration finds the previous report's context resident.
        let again = task_key(OneOffTask::Report, Some(("patient-a", "p1:abc")));
        let plan = plan_slot(&[(&chat, 1), (&report, 2)], &again, 2);
        assert_eq!(plan.action, SlotAction::Hit(1));
        assert!(plan.evict.is_empty());

        // Previously every distinct prompt was a new digest key and missed.
        let first = one_off("first");
        assert_eq!(
            plan_slot(&[(&chat, 1), (&first, 2)], &one_off("second"), 2).action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: true
            }
        );
    }

    #[test]
    fn switching_task_family_keeps_the_prefix_only_for_the_same_record() {
        let report = task_key(OneOffTask::Report, Some(("patient-a", "p1:abc")));
        let letter = task_key(OneOffTask::Letter, Some(("patient-a", "p1:abc")));
        assert!(!report.same_logical_context(&letter));
        assert_eq!(
            plan_slot(&[(&report, 1)], &letter, 1).action,
            SlotAction::Reuse {
                index: 0,
                keep_prefix: true
            }
        );
        // Both stay resident when the pool has room.
        assert_eq!(
            plan_slot(&[(&report, 1)], &letter, 2).action,
            SlotAction::Create
        );

        let other_patient = task_key(OneOffTask::Letter, Some(("patient-b", "p1:abc")));
        let other_revision = task_key(OneOffTask::Letter, Some(("patient-a", "p1:new")));
        let unbound = task_key(OneOffTask::Extract, None);
        for next in [&other_patient, &other_revision, &unbound] {
            assert_eq!(
                plan_slot(&[(&report, 1)], next, 1).action,
                SlotAction::Reuse {
                    index: 0,
                    keep_prefix: false
                }
            );
        }

        // An agent conversation on the same record revision may share the
        // system prompt and patient context with a task context.
        let mut chat = conversation("chat", "patient-a");
        chat.patient_revision = Some("p1:abc".into());
        assert_eq!(
            plan_slot(&[(&chat, 1)], &report, 1).action,
            SlotAction::Reuse {
                index: 0,
                keep_prefix: true
            }
        );
    }

    #[test]
    fn task_contexts_are_evicted_before_conversations() {
        let chat = conversation("chat", "patient-a");
        let report = task_key(OneOffTask::Report, Some(("patient-a", "p1:abc")));
        let plan = plan_slot(
            &[(&chat, 1), (&report, 5)],
            &conversation("chat-2", "patient-b"),
            2,
        );
        assert_eq!(
            plan.action,
            SlotAction::Reuse {
                index: 1,
                keep_prefix: false
            }
        );
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

    #[test]
    fn static_prefix_is_the_common_token_prefix_minus_one() {
        assert_eq!(static_prefix_len(&[1, 2, 3, 4, 9], &[1, 2, 3, 4, 8]), 3);
        // One probe a prefix of the other.
        assert_eq!(static_prefix_len(&[1, 2, 3], &[1, 2, 3, 4]), 2);
        assert_eq!(static_prefix_len(&[7, 2], &[8, 2]), 0);
        assert_eq!(static_prefix_len::<u32>(&[], &[1, 2]), 0);
        assert_eq!(static_prefix_len::<u32>(&[], &[]), 0);
        assert_eq!(static_prefix_len(&[1], &[1]), 0);
    }

    #[test]
    fn prefill_is_split_at_cuts_inside_the_uncached_range() {
        assert_eq!(prefill_bounds(0, 100, &[]), vec![0, 100]);
        assert_eq!(prefill_bounds(0, 100, &[Some(40), None]), vec![0, 40, 100]);
        assert_eq!(
            prefill_bounds(10, 100, &[Some(90), Some(40)]),
            vec![10, 40, 90, 100]
        );
        // Cuts at or outside the range do not add a segment.
        assert_eq!(
            prefill_bounds(40, 100, &[Some(40), Some(100), Some(5)]),
            vec![40, 100]
        );
        assert_eq!(
            prefill_bounds(10, 100, &[Some(50), Some(50)]),
            vec![10, 50, 100]
        );
        assert_eq!(prefill_bounds(7, 7, &[Some(7)]), vec![7]);
    }

    #[test]
    fn patient_markers_are_detected() {
        assert!(contains_patient_marker(
            "Tools …\n\nDu arbeitest im Kontext von Patient-ID: 42."
        ));
        assert!(contains_patient_marker("Aktuelle Patientendaten (bereits"));
        assert!(!contains_patient_marker(
            "<tools>\nget_patient(...)\n</tools>"
        ));
    }

    fn static_prefix(family: &str) -> StaticPrefix {
        let probe = |system: &str| PrefixProbe {
            system: system.into(),
            user: "?".into(),
        };
        StaticPrefix {
            family: family.into(),
            prompt_version: AGENT_PROMPT_VERSION.into(),
            probes: [probe("a"), probe("b")],
        }
    }

    #[test]
    fn prefix_snapshot_key_tracks_profile_and_prompt_but_not_patient() {
        let prefix = static_prefix("agent:patient");
        let context = key("patient-a", "revision-1", "model-a");
        let base = PrefixSnapshotKey::new(&context, &prefix);
        // Another patient, revision or conversation shares the snapshot.
        assert_eq!(
            base,
            PrefixSnapshotKey::new(&key("patient-b", "revision-9", "model-a"), &prefix)
        );
        assert_eq!(
            base,
            PrefixSnapshotKey::new(&conversation("other", "patient-c"), &prefix)
        );

        assert_ne!(
            base,
            PrefixSnapshotKey::new(&key("patient-a", "revision-1", "model-b"), &prefix)
        );
        let mut template = context.clone();
        template.chat_template_hash = "other-template".into();
        assert_ne!(base, PrefixSnapshotKey::new(&template, &prefix));
        let mut kv = context.clone();
        kv.kv_config_hash = "q8:512:enabled".into();
        assert_ne!(base, PrefixSnapshotKey::new(&kv, &prefix));
        let mut n_ctx = context.clone();
        n_ctx.context_size = 32_768;
        assert_ne!(base, PrefixSnapshotKey::new(&n_ctx, &prefix));
        let mut adapter = context.clone();
        adapter.adapter_hash = "adapter-a".into();
        assert_ne!(base, PrefixSnapshotKey::new(&adapter, &prefix));

        let mut bumped = prefix.clone();
        bumped.prompt_version = format!("{AGENT_PROMPT_VERSION}-next");
        assert_ne!(base, PrefixSnapshotKey::new(&context, &bumped));
        assert_ne!(
            base,
            PrefixSnapshotKey::new(&context, &static_prefix("agent:global"))
        );

        assert!(base.same_runtime_profile(&context));
        assert!(!base.same_runtime_profile(&kv));
    }

    fn snapshot_key(family: &str) -> PrefixSnapshotKey {
        PrefixSnapshotKey::new(
            &key("patient-a", "revision-1", "model-a"),
            &static_prefix(family),
        )
    }

    #[test]
    fn snapshot_lookup_needs_the_same_key_and_a_strict_token_prefix() {
        let mut store = PrefixSnapshotStore::<u32, &str>::default();
        let agent = snapshot_key("agent:patient");
        assert!(store.insert(agent.clone(), vec![1, 2, 3], "state", 10, 100));

        let found = store.find(&agent, &[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(store.get(found).state, "state");
        // The prompt must continue past the prefix.
        assert_eq!(store.find(&agent, &[1, 2, 3]), None);
        assert_eq!(store.find(&agent, &[1, 2, 9, 4]), None);
        // Another model, KV config or prompt version never restores it.
        let mut model = agent.clone();
        model.model_hash = "model-b".into();
        assert_eq!(store.find(&model, &[1, 2, 3, 4]), None);
        let mut kv = agent.clone();
        kv.kv_config_hash = "q8:512:enabled".into();
        assert_eq!(store.find(&kv, &[1, 2, 3, 4]), None);
        let mut version = agent.clone();
        version.prompt_version = "agent-next".into();
        assert_eq!(store.find(&version, &[1, 2, 3, 4]), None);

        // With two variants, the longest matching one wins.
        assert!(store.insert(agent.clone(), vec![1, 2, 3, 4], "longer", 10, 100));
        let found = store.find(&agent, &[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(store.get(found).state, "longer");
        let found = store.find(&agent, &[1, 2, 3, 9]).unwrap();
        assert_eq!(store.get(found).state, "state");

        store.remove(found);
        assert!(!store.contains(&agent, &[1, 2, 3]));
        assert!(store.contains(&agent, &[1, 2, 3, 4]));
    }

    #[test]
    fn snapshot_store_evicts_least_recently_used_within_caps() {
        let mut store = PrefixSnapshotStore::<u32, u32>::default();
        let agent = snapshot_key("agent:patient");
        assert!(store.insert(agent.clone(), vec![1], 1, 10, 1_000));
        assert!(store.insert(agent.clone(), vec![2], 2, 10, 1_000));
        // Re-inserting the same tokens replaces, not duplicates.
        assert!(store.insert(agent.clone(), vec![2], 3, 10, 1_000));
        assert_eq!(store.bytes(), 20);
        // A third variant for one key evicts that key's least recently used.
        store.find(&agent, &[1, 0]);
        assert!(store.insert(agent.clone(), vec![3], 4, 10, 1_000));
        assert!(store.contains(&agent, &[1]));
        assert!(!store.contains(&agent, &[2]));
        assert!(store.contains(&agent, &[3]));

        // The total cap evicts across keys, least recently used first.
        let global = snapshot_key("agent:global");
        let report = snapshot_key("system:de");
        let letter = snapshot_key("system:fr");
        assert!(store.insert(global, vec![5], 5, 10, 1_000));
        assert!(store.insert(report, vec![6], 6, 10, 1_000));
        assert_eq!(store.bytes(), 40);
        assert!(store.insert(letter.clone(), vec![7], 7, 10, 1_000));
        assert_eq!(store.bytes(), 40);
        assert!(!store.contains(&agent, &[1]));
        assert!(store.contains(&letter, &[7]));
    }

    #[test]
    fn snapshot_store_respects_the_byte_budget() {
        let mut store = PrefixSnapshotStore::<u32, u32>::default();
        let agent = snapshot_key("agent:patient");
        let report = snapshot_key("system:de");
        // Larger than the budget: not stored at all.
        assert!(!store.insert(agent.clone(), vec![1], 1, 200, 100));
        assert_eq!(store.bytes(), 0);
        assert!(store.insert(agent.clone(), vec![1], 1, 60, 100));
        // Making room evicts the older snapshot.
        assert!(store.insert(report, vec![2], 2, 60, 100));
        assert!(!store.contains(&agent, &[1]));
        assert_eq!(store.bytes(), 60);

        // A profile change drops snapshots of the old profile.
        let mut kv = key("patient-a", "revision-1", "model-a");
        kv.kv_config_hash = "q8:512:enabled".into();
        store.retain(|snapshot| snapshot.same_runtime_profile(&kv));
        assert_eq!(store.bytes(), 0);
    }

    #[test]
    fn with_static_prefix_keeps_the_context_identity() {
        let session = InferenceSession::agent("chat", Some("p".into()), Some("r".into()));
        let seeded = session
            .clone()
            .with_static_prefix(Some(static_prefix("agent:patient")));
        assert_eq!(seeded.conversation_id, session.conversation_id);
        assert_eq!(seeded.patient(), session.patient());
        assert_eq!(seeded.prompt_version, session.prompt_version);
        assert!(seeded.static_prefix.is_some());
        assert!(session.with_static_prefix(None).static_prefix.is_none());
    }
}

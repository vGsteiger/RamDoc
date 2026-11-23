//! Model-facing view of tool results.
//!
//! A tool result has two consumers with different needs:
//!
//! - **Persistence and UI** need the full JSON. `write_report` proposals carry
//!   the whole report, and both `report_proposal()` (draft versions) and the
//!   confirmation-gated proposal UI parse it.
//! - **The model** only needs enough to continue the conversation, within a
//!   bounded context budget.
//!
//! [`model_facing`] derives the second from the first. It never cuts the
//! serialized JSON at a byte position: it shortens the parsed value
//! structurally (long strings, long arrays, wide objects), so the model always
//! sees valid JSON. Confirmation-gated proposals collapse to a short receipt so
//! the model does not re-type a report it just drafted.

use serde_json::{json, Map, Value};

/// Maximum bytes of a tool result that are fed back into the LLM.
pub const MODEL_TOOL_RESULT_BUDGET: usize = 4_000;

/// Status every confirmation-gated write proposal carries (`tools::proposed_write`).
const PENDING_STATUS: &str = "pending_clinician_confirmation";

/// Proposal fields whose string value is at most this many chars are kept
/// verbatim in the receipt; longer ones are replaced by a `<field>_chars` count.
const RECEIPT_FIELD_MAX_CHARS: usize = 200;

/// Successively tighter (max string chars, max array/object entries) limits.
const SHRINK_LEVELS: &[(usize, usize)] = &[
    (2_000, 100),
    (1_000, 50),
    (500, 20),
    (250, 10),
    (120, 5),
    (60, 3),
    (30, 1),
];

/// The model-facing form of a persisted/full tool result string.
///
/// Always returns valid JSON no longer than `MODEL_TOOL_RESULT_BUDGET` bytes.
pub fn model_facing(full: &str) -> String {
    model_facing_with_budget(full, MODEL_TOOL_RESULT_BUDGET)
}

fn model_facing_with_budget(full: &str, budget: usize) -> String {
    let value: Value = match serde_json::from_str(full) {
        Ok(v) => v,
        // Not JSON (e.g. a legacy row persisted already cut mid-string):
        // wrap a char-safe preview so the model still sees valid JSON.
        Err(_) => return unparseable_preview(full, budget),
    };

    if let Some(receipt) = proposal_receipt(&value) {
        let s = receipt.to_string();
        if s.len() <= budget {
            return s;
        }
        // Pathological receipt (huge short-field count); fall through to the
        // generic shrinker using the receipt as input.
        return shrink_to_budget(&receipt, budget);
    }

    let s = value.to_string();
    if s.len() <= budget {
        return s;
    }
    shrink_to_budget(&value, budget)
}

/// For a `proposed_write` result, a short receipt; otherwise `None`.
fn proposal_receipt(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    if obj.get("status").and_then(Value::as_str) != Some(PENDING_STATUS) {
        return None;
    }
    let mut receipt = Map::new();
    receipt.insert("status".into(), json!(PENDING_STATUS));
    if let Some(action) = obj.get("action") {
        receipt.insert("action".into(), action.clone());
    }
    if let Some(proposal) = obj.get("proposal").and_then(Value::as_object) {
        for (key, field) in proposal {
            match field {
                Value::Null => {}
                Value::String(s) if s.chars().count() > RECEIPT_FIELD_MAX_CHARS => {
                    receipt.insert(format!("{key}_chars"), json!(s.chars().count()));
                }
                Value::Array(_) | Value::Object(_) => {
                    let len = field.to_string().chars().count();
                    if len > RECEIPT_FIELD_MAX_CHARS {
                        receipt.insert(format!("{key}_chars"), json!(len));
                    } else {
                        receipt.insert(key.clone(), field.clone());
                    }
                }
                _ => {
                    receipt.insert(key.clone(), field.clone());
                }
            }
        }
    }
    receipt.insert(
        "note".into(),
        json!(
            "Vorschlag liegt der Fachperson zur Bestätigung in der Benutzeroberfläche vor \
             und ist noch nicht gespeichert. Den Inhalt nicht wiederholen."
        ),
    );
    Some(Value::Object(receipt))
}

fn shrink_to_budget(value: &Value, budget: usize) -> String {
    for &(max_chars, max_items) in SHRINK_LEVELS {
        let wrapped = json!({
            "truncated": true,
            "result": shrink(value, max_chars, max_items),
        });
        let s = wrapped.to_string();
        if s.len() <= budget {
            return s;
        }
    }
    // Even the tightest structural shrink does not fit (very deep nesting).
    let s = json!({
        "truncated": true,
        "note": "Ergebnis zu umfangreich für den Kontext.",
        "chars": value.to_string().chars().count(),
    })
    .to_string();
    if s.len() <= budget {
        s
    } else {
        r#"{"truncated":true}"#.to_string()
    }
}

fn shrink(value: &Value, max_chars: usize, max_items: usize) -> Value {
    match value {
        Value::String(s) => Value::String(shorten_str(s, max_chars)),
        Value::Array(items) => {
            let mut out: Vec<Value> = items
                .iter()
                .take(max_items)
                .map(|v| shrink(v, max_chars, max_items))
                .collect();
            if items.len() > max_items {
                out.push(json!(format!(
                    "[… {} weitere Einträge ausgelassen]",
                    items.len() - max_items
                )));
            }
            Value::Array(out)
        }
        Value::Object(map) => {
            let mut out = Map::new();
            // Wide objects get a more generous cap than arrays: a record's
            // fields are usually all relevant, list entries are repetitive.
            let max_fields = max_items.saturating_mul(4).max(8);
            for (k, v) in map.iter().take(max_fields) {
                out.insert(k.clone(), shrink(v, max_chars, max_items));
            }
            if map.len() > max_fields {
                out.insert("_omitted_fields".into(), json!(map.len() - max_fields));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Keep at most `max_chars` chars, marking the cut with an ellipsis and the
/// original length. Works on chars, so multi-byte text is never split.
fn shorten_str(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    format!("{head}… [{total} Zeichen, gekürzt]")
}

fn unparseable_preview(full: &str, budget: usize) -> String {
    let total = full.chars().count();
    // Escaping can expand the preview (quotes, backslashes, control chars),
    // so halve until the serialized form fits.
    let mut take = budget / 2;
    loop {
        let preview: String = full.chars().take(take).collect();
        let s = json!({
            "truncated": true,
            "unparseable": true,
            "chars": total,
            "preview": preview,
        })
        .to_string();
        if s.len() <= budget || take == 0 {
            return s;
        }
        take /= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn long_report(chars: usize) -> String {
        // Non-ASCII on purpose: umlauts and ß are multi-byte in UTF-8.
        "Befund: Patientin äußert Ängste, Schlafstörung. "
            .chars()
            .cycle()
            .take(chars)
            .collect()
    }

    fn write_report_result(content: &str) -> String {
        json!({
            "status": PENDING_STATUS,
            "action": "create_report",
            "proposal": {
                "patient_id": "p1",
                "report_type": "befundbericht",
                "content": content,
                "model_name": null,
                "prompt_hash": null,
                "session_ids": null,
            },
            "message": "Dieser Vorschlag wurde nicht gespeichert.",
        })
        .to_string()
    }

    #[test]
    fn small_result_is_unchanged() {
        let full = json!({"patients": [{"id": "p1", "name": "Müller"}]}).to_string();
        assert_eq!(model_facing(&full), full);
    }

    #[test]
    fn oversized_report_proposal_full_form_stays_parseable() {
        // The full string is what gets persisted; nothing in this module
        // mutates it, and it must parse back to the whole report.
        let content = long_report(12_000);
        let full = write_report_result(&content);
        assert!(full.len() > MODEL_TOOL_RESULT_BUDGET);
        let parsed: Value = serde_json::from_str(&full).unwrap();
        assert_eq!(parsed["proposal"]["content"].as_str().unwrap(), content);
    }

    #[test]
    fn report_proposal_yields_receipt() {
        let content = long_report(12_000);
        let full = write_report_result(&content);
        let facing = model_facing(&full);
        assert!(facing.len() <= MODEL_TOOL_RESULT_BUDGET);
        let v: Value = serde_json::from_str(&facing).unwrap();
        assert_eq!(v["status"], PENDING_STATUS);
        assert_eq!(v["action"], "create_report");
        assert_eq!(v["report_type"], "befundbericht");
        assert_eq!(v["patient_id"], "p1");
        assert_eq!(v["content_chars"], 12_000);
        assert!(v.get("content").is_none());
        assert!(v.get("model_name").is_none(), "null fields are dropped");
        assert!(!facing.contains("Schlafstörung"));
    }

    #[test]
    fn short_proposal_fields_are_kept_in_receipt() {
        let full = json!({
            "status": PENDING_STATUS,
            "action": "create_diagnosis",
            "proposal": {"patient_id": "p1", "icd10_code": "F32.1", "notes": "kurz"},
            "message": "x",
        })
        .to_string();
        let v: Value = serde_json::from_str(&model_facing(&full)).unwrap();
        assert_eq!(v["icd10_code"], "F32.1");
        assert_eq!(v["notes"], "kurz");
    }

    #[test]
    fn oversized_generic_result_is_valid_json_within_budget() {
        let items: Vec<Value> = (0..200)
            .map(|i| json!({"id": i, "titel": long_report(300), "größe": "ß".repeat(50)}))
            .collect();
        let full = json!({"results": items}).to_string();
        assert!(full.len() > MODEL_TOOL_RESULT_BUDGET);
        let facing = model_facing(&full);
        assert!(facing.len() <= MODEL_TOOL_RESULT_BUDGET, "{}", facing.len());
        let v: Value = serde_json::from_str(&facing).unwrap();
        assert_eq!(v["truncated"], true);
        assert!(v["result"]["results"].is_array());
    }

    #[test]
    fn single_huge_multibyte_string_is_shortened_on_char_boundary() {
        let full = json!({"body": "€".repeat(10_000)}).to_string();
        let facing = model_facing(&full);
        assert!(facing.len() <= MODEL_TOOL_RESULT_BUDGET);
        let v: Value = serde_json::from_str(&facing).unwrap();
        assert!(v["result"]["body"].as_str().unwrap().starts_with('€'));
    }

    #[test]
    fn unparseable_legacy_row_becomes_valid_json() {
        // A row persisted before #552: JSON cut mid-string.
        let full = write_report_result(&long_report(12_000));
        let cut: String = full.chars().take(3_000).collect();
        let facing = model_facing(&cut);
        assert!(facing.len() <= MODEL_TOOL_RESULT_BUDGET);
        let v: Value = serde_json::from_str(&facing).unwrap();
        assert_eq!(v["unparseable"], true);
    }

    #[test]
    fn falls_back_when_structure_cannot_fit() {
        let full = json!({"a": "x".repeat(500)}).to_string();
        let facing = model_facing_with_budget(&full, 40);
        let v: Value = serde_json::from_str(&facing).unwrap();
        assert_eq!(v["truncated"], true);
    }

    #[test]
    fn shorten_str_counts_chars_not_bytes() {
        assert_eq!(shorten_str("äöü", 3), "äöü");
        assert_eq!(shorten_str("äöüß", 2), "äö… [4 Zeichen, gekürzt]");
    }
}

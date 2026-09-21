use crate::error::AppError;
use crate::llm::agent::{run_agent_loop, AgentScope, AgentTurnInput};
use crate::llm::context_cache::InferenceSession;
use crate::llm::engine::AgentMessage;
use crate::llm::ThinkingEffort;
use crate::models::chat::{
    self, ChatDraftVersion, ChatMessageRow, ChatSession, CreateChatDraftVersion, CreateChatMessage,
};
use crate::models::patient;
use crate::state::{llm_lock_poisoned, AppState, AuthState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Returned by `run_agent_turn` — the complete result of one user interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnResult {
    pub session_id: String,
    pub final_answer: String,
    pub tool_calls_made: Vec<crate::llm::agent::ExecutedToolCall>,
}

fn require_unlocked(state: &AppState) -> Result<(), AppError> {
    let auth = state
        .auth
        .lock()
        .map_err(|_| AppError::Llm("Auth state poisoned".to_string()))?;
    if !matches!(*auth, AuthState::Unlocked { .. }) {
        return Err(AppError::AuthRequired);
    }
    Ok(())
}

/// The only chat artifact eligible for durable revisions is a report proposal
/// produced by the validated `write_report` tool.  Do not infer this from
/// assistant text: tool-result rows are the typed, persisted boundary.
fn report_proposal(message: &ChatMessageRow) -> Result<(String, String), AppError> {
    if message.role != "tool_result" || message.tool_name.as_deref() != Some("write_report") {
        return Err(AppError::Validation(
            "Draft revisions require a report tool result".to_string(),
        ));
    }
    let value: serde_json::Value = serde_json::from_str(&message.content)
        .map_err(|_| AppError::Validation("Invalid report tool result".to_string()))?;
    let proposal = value
        .get("proposal")
        .and_then(serde_json::Value::as_object)
        .filter(|_| {
            value.get("status").and_then(serde_json::Value::as_str)
                == Some("pending_clinician_confirmation")
        })
        .filter(|_| {
            value.get("action").and_then(serde_json::Value::as_str) == Some("create_report")
        })
        .ok_or_else(|| AppError::Validation("Missing report proposal".to_string()))?;
    let patient_id = proposal
        .get("patient_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Validation("Missing report proposal patient".to_string()))?;
    let content = proposal
        .get("content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Validation("Missing report proposal content".to_string()))?;
    Ok((patient_id.to_string(), content.to_string()))
}

fn validate_draft_message_scope(
    conn: &rusqlite::Connection,
    tool_result_message_id: &str,
) -> Result<ChatMessageRow, AppError> {
    let message = chat::get_chat_message(conn, tool_result_message_id)?;
    let (proposal_patient_id, _) = report_proposal(&message)?;
    let session = chat::get_chat_session(conn, &message.session_id)?;
    // Keep the existing patient-session boundary intact even for revision-only
    // writes. A global session retains its existing capabilities.
    if session.scope == "patient" && session.patient_id.as_deref() != Some(&proposal_patient_id) {
        return Err(AppError::Validation(
            "Patient scope: draft proposal belongs to a different patient".to_string(),
        ));
    }
    Ok(message)
}

/// Main command: persist user message, run agent loop, persist results.
#[tauri::command]
pub async fn run_agent_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    user_message: String,
    thinking_effort: Option<ThinkingEffort>,
) -> Result<AgentTurnResult, AppError> {
    require_unlocked(&state)?;

    // Acquire engine (must be loaded)
    let engine = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        llm.as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))
            .map(Arc::clone)?
    };

    // The router is an optional independent runtime. Its failure or memory
    // admission denial must not turn a normal chat into an error: the agent
    // uses the writing engine for tool probes in that explicitly observable
    // fallback state.
    let router_engine = match crate::commands::llm::ensure_router_engine(&state).await {
        Ok(engine) => engine,
        Err(error) => {
            log::warn!(
                "Dedicated tool router unavailable; using writing engine for probes: {error}"
            );
            None
        }
    };
    let dedicated_router = router_engine.is_some();

    let pool = state.get_db()?;

    // Determine scope from session and pre-fetch patient context if applicable
    let (scope, patient_context, patient_revision, history) = {
        let conn = pool.conn()?;
        let session = chat::get_chat_session(&conn, &session_id)?;
        let scope = if session.scope == "patient" {
            match &session.patient_id {
                Some(pid) => AgentScope::Patient {
                    patient_id: pid.clone(),
                },
                None => AgentScope::Global,
            }
        } else {
            AgentScope::Global
        };

        // Pre-fetch patient data so the model knows who it's talking about
        // without needing a get_patient tool call.
        let patient_record = if let AgentScope::Patient { ref patient_id } = scope {
            patient::get_patient(&conn, patient_id).ok()
        } else {
            None
        };
        // The agent's tools read sessions, medications and documents too, so the
        // cache key has to cover the whole record — `patients.updated_at` alone
        // would keep a context alive across an edited session note.
        let patient_revision = match &scope {
            AgentScope::Patient { patient_id } => Some(
                crate::llm::evidence::provenance::patient_revision(&conn, patient_id)?,
            ),
            AgentScope::Global => None,
        };
        let patient_context = patient_record
            .as_ref()
            .and_then(|p| serde_json::to_string(p).ok());

        // Load existing messages as AgentMessages
        let msgs = chat::list_chat_messages(&conn, &session_id)?;
        let history: Vec<AgentMessage> = msgs
            .iter()
            .map(|m| AgentMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        // Persist user message
        chat::append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: session_id.clone(),
                role: "user".to_string(),
                content: user_message.clone(),
                tool_name: None,
                tool_args_json: None,
                tool_result_for: None,
            },
        )?;

        (scope, patient_context, patient_revision, history)
    };

    // Run the agent loop on a blocking thread
    let app_clone = app.clone();
    let session_id_clone = session_id.clone();
    let inference_session = InferenceSession::agent(
        session_id.clone(),
        match &scope {
            AgentScope::Patient { patient_id } => Some(patient_id.clone()),
            AgentScope::Global => None,
        },
        patient_revision,
    );
    let result = tokio::task::spawn_blocking(move || {
        run_agent_loop(
            &app_clone,
            &engine,
            router_engine.as_ref(),
            &pool,
            AgentTurnInput {
                inference_session,
                scope,
                patient_context,
                history,
                user_message,
                thinking_effort: thinking_effort.unwrap_or_default(),
            },
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")));

    let result = match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) | Err(error) => {
            let message = error.to_string();
            let _ = app.emit(
                "agent-error-session",
                serde_json::json!({"session_id": session_id, "message": message}),
            );
            return Err(error);
        }
    };

    if let Some(average_ms) = result
        .router_probe_duration_ms
        .checked_div(result.router_probe_count)
    {
        let per_probe = std::time::Duration::from_millis(average_ms);
        for _ in 0..result.router_probe_count {
            state.record_router_probe(dedicated_router, per_probe);
        }
    }

    // Persist tool calls and assistant answer
    {
        let pool2 = state.get_db()?;
        let conn = pool2.conn()?;
        for tc in &result.tool_calls_made {
            // tool_call message
            let tc_msg = chat::append_chat_message(
                &conn,
                &CreateChatMessage {
                    session_id: session_id_clone.clone(),
                    role: "tool_call".to_string(),
                    content: tc.args_json.clone(),
                    tool_name: Some(tc.name.clone()),
                    tool_args_json: Some(tc.args_json.clone()),
                    tool_result_for: None,
                },
            )?;
            // tool_result message
            let result_message = chat::append_chat_message(
                &conn,
                &CreateChatMessage {
                    session_id: session_id_clone.clone(),
                    role: "tool_result".to_string(),
                    content: tc.result_json.clone(),
                    tool_name: Some(tc.name.clone()),
                    tool_args_json: None,
                    tool_result_for: Some(tc_msg.id.clone()),
                },
            )?;
            // `write_report` has already returned a confirmation-gated
            // proposal. Preserve its AI-produced initial version while it is
            // still an unsaved artifact, rather than creating a report row.
            if let Ok((_, content)) = report_proposal(&result_message) {
                chat::append_chat_draft_version(
                    &conn,
                    &CreateChatDraftVersion {
                        tool_result_message_id: result_message.id,
                        content,
                        origin: "ai".to_string(),
                        claim_resolutions_json: "[]".to_string(),
                    },
                )?;
            }
        }
        // Persist final assistant message
        chat::append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: session_id_clone.clone(),
                role: "assistant".to_string(),
                content: result.final_answer.clone(),
                tool_name: None,
                tool_args_json: None,
                tool_result_for: None,
            },
        )?;
    }

    let _ = app.emit(
        "agent-done",
        serde_json::json!({
            "final_answer": result.final_answer,
            "session_id": session_id_clone,
        }),
    );

    Ok(AgentTurnResult {
        session_id: session_id_clone,
        final_answer: result.final_answer,
        tool_calls_made: result.tool_calls_made,
    })
}

#[tauri::command]
pub async fn create_chat_session(
    state: State<'_, AppState>,
    scope: String,
    patient_id: Option<String>,
    title: Option<String>,
) -> Result<ChatSession, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::create_chat_session(
        &conn,
        &scope,
        patient_id.as_deref(),
        &title.unwrap_or_else(|| "New Chat".to_string()),
    )
}

#[tauri::command]
pub async fn get_or_create_patient_chat_session(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<ChatSession, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::get_or_create_patient_session(&conn, &patient_id)
}

#[tauri::command]
pub async fn list_chat_sessions(
    state: State<'_, AppState>,
    scope: String,
    patient_id: Option<String>,
) -> Result<Vec<ChatSession>, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::list_chat_sessions(&conn, &scope, patient_id.as_deref(), 50)
}

#[tauri::command]
pub async fn delete_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::delete_chat_session(&conn, &session_id)
}

#[tauri::command]
pub async fn get_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessageRow>, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::list_chat_messages(&conn, &session_id)
}

#[tauri::command]
pub async fn list_chat_draft_versions(
    state: State<'_, AppState>,
    tool_result_message_id: String,
) -> Result<Vec<ChatDraftVersion>, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    validate_draft_message_scope(&conn, &tool_result_message_id)?;
    chat::list_chat_draft_versions(&conn, &tool_result_message_id)
}

#[tauri::command]
pub async fn append_chat_draft_version(
    state: State<'_, AppState>,
    input: CreateChatDraftVersion,
) -> Result<ChatDraftVersion, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    validate_draft_message_scope(&conn, &input.tool_result_message_id)?;
    chat::append_chat_draft_version(&conn, &input)
}

#[tauri::command]
pub async fn rename_chat_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<ChatSession, AppError> {
    require_unlocked(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    chat::update_chat_session_title(&conn, &session_id, &title)
}

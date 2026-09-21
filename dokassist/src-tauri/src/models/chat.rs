use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub scope: String,
    pub patient_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args_json: Option<String>,
    pub tool_result_for: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChatMessage {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args_json: Option<String>,
    pub tool_result_for: Option<String>,
}

/// An immutable revision of an unsaved chat draft.  Saving a report is a
/// separate, explicit clinician action; this table only preserves revisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDraftVersion {
    pub id: String,
    pub tool_result_message_id: String,
    pub version_number: i64,
    pub content: String,
    pub origin: String,
    pub claim_resolutions_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateChatDraftVersion {
    pub tool_result_message_id: String,
    pub content: String,
    pub origin: String,
    #[serde(default = "default_claim_resolutions")]
    pub claim_resolutions_json: String,
}

fn default_claim_resolutions() -> String {
    "[]".to_string()
}

fn row_to_session(row: &Row) -> Result<ChatSession, rusqlite::Error> {
    Ok(ChatSession {
        id: row.get(0)?,
        scope: row.get(1)?,
        patient_id: row.get(2)?,
        title: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn row_to_message(row: &Row) -> Result<ChatMessageRow, rusqlite::Error> {
    Ok(ChatMessageRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        tool_name: row.get(4)?,
        tool_args_json: row.get(5)?,
        tool_result_for: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn row_to_draft_version(row: &Row) -> Result<ChatDraftVersion, rusqlite::Error> {
    Ok(ChatDraftVersion {
        id: row.get(0)?,
        tool_result_message_id: row.get(1)?,
        version_number: row.get(2)?,
        content: row.get(3)?,
        origin: row.get(4)?,
        claim_resolutions_json: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub fn create_chat_session(
    conn: &Connection,
    scope: &str,
    patient_id: Option<&str>,
    title: &str,
) -> Result<ChatSession, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO chat_sessions (id, scope, patient_id, title) VALUES (?, ?, ?, ?)",
        params![id, scope, patient_id, title],
    )?;
    get_chat_session(conn, &id)
}

pub fn get_chat_session(conn: &Connection, id: &str) -> Result<ChatSession, AppError> {
    conn.query_row(
        "SELECT id, scope, patient_id, title, created_at, updated_at FROM chat_sessions WHERE id = ?",
        params![id],
        row_to_session,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Chat session not found: {}", id))
        }
        other => AppError::from(other),
    })
}

pub fn get_or_create_patient_session(
    conn: &Connection,
    patient_id: &str,
) -> Result<ChatSession, AppError> {
    let existing = conn.query_row(
        "SELECT id, scope, patient_id, title, created_at, updated_at
         FROM chat_sessions WHERE scope = 'patient' AND patient_id = ?
         ORDER BY created_at DESC LIMIT 1",
        params![patient_id],
        row_to_session,
    );
    match existing {
        Ok(session) => Ok(session),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            create_chat_session(conn, "patient", Some(patient_id), "Chat")
        }
        Err(e) => Err(AppError::from(e)),
    }
}

pub fn list_chat_sessions(
    conn: &Connection,
    scope: &str,
    patient_id: Option<&str>,
    limit: u32,
) -> Result<Vec<ChatSession>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, scope, patient_id, title, created_at, updated_at
         FROM chat_sessions
         WHERE scope = ? AND (patient_id = ? OR ? IS NULL)
         ORDER BY updated_at DESC
         LIMIT ?",
    )?;
    let sessions = stmt
        .query_map(
            params![scope, patient_id, patient_id, limit],
            row_to_session,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sessions)
}

pub fn delete_chat_session(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows = conn.execute("DELETE FROM chat_sessions WHERE id = ?", params![id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Chat session not found: {}",
            id
        )));
    }
    Ok(())
}

pub fn update_chat_session_title(
    conn: &Connection,
    id: &str,
    title: &str,
) -> Result<ChatSession, AppError> {
    get_chat_session(conn, id)?;
    conn.execute(
        "UPDATE chat_sessions SET title = ? WHERE id = ?",
        params![title, id],
    )?;
    get_chat_session(conn, id)
}

pub fn append_chat_message(
    conn: &Connection,
    msg: &CreateChatMessage,
) -> Result<ChatMessageRow, AppError> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO chat_messages (id, session_id, role, content, tool_name, tool_args_json, tool_result_for)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            msg.session_id,
            msg.role,
            msg.content,
            msg.tool_name,
            msg.tool_args_json,
            msg.tool_result_for,
        ],
    )?;
    // Touch the session updated_at
    conn.execute(
        "UPDATE chat_sessions SET updated_at = datetime('now') WHERE id = ?",
        params![msg.session_id],
    )?;
    conn.query_row(
        "SELECT id, session_id, role, content, tool_name, tool_args_json, tool_result_for, created_at
         FROM chat_messages WHERE id = ?",
        params![id],
        row_to_message,
    )
    .map_err(AppError::from)
}

pub fn list_chat_messages(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<ChatMessageRow>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, role, content, tool_name, tool_args_json, tool_result_for, created_at
         FROM chat_messages WHERE session_id = ? ORDER BY created_at ASC",
    )?;
    let msgs = stmt
        .query_map(params![session_id], row_to_message)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(msgs)
}

pub fn get_chat_message(conn: &Connection, id: &str) -> Result<ChatMessageRow, AppError> {
    conn.query_row(
        "SELECT id, session_id, role, content, tool_name, tool_args_json, tool_result_for, created_at
         FROM chat_messages WHERE id = ?",
        params![id],
        row_to_message,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Chat message not found: {id}"))
        }
        other => AppError::from(other),
    })
}

pub fn append_chat_draft_version(
    conn: &Connection,
    input: &CreateChatDraftVersion,
) -> Result<ChatDraftVersion, AppError> {
    if !matches!(input.origin.as_str(), "ai" | "manual") {
        return Err(AppError::Validation(
            "Invalid draft version origin".to_string(),
        ));
    }
    // Store valid JSON so resolution records retain a stable, typed shape.
    serde_json::from_str::<serde_json::Value>(&input.claim_resolutions_json).map_err(|_| {
        AppError::Validation("claim_resolutions_json must be valid JSON".to_string())
    })?;
    let version_number: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM chat_draft_versions
         WHERE tool_result_message_id = ?",
        params![input.tool_result_message_id],
        |row| row.get(0),
    )?;
    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO chat_draft_versions
         (id, tool_result_message_id, version_number, content, origin, claim_resolutions_json)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.tool_result_message_id,
            version_number,
            input.content,
            input.origin,
            input.claim_resolutions_json,
        ],
    )?;
    conn.query_row(
        "SELECT id, tool_result_message_id, version_number, content, origin, claim_resolutions_json, created_at
         FROM chat_draft_versions WHERE id = ?",
        params![id],
        row_to_draft_version,
    )
    .map_err(AppError::from)
}

pub fn list_chat_draft_versions(
    conn: &Connection,
    tool_result_message_id: &str,
) -> Result<Vec<ChatDraftVersion>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, tool_result_message_id, version_number, content, origin, claim_resolutions_json, created_at
         FROM chat_draft_versions WHERE tool_result_message_id = ? ORDER BY version_number ASC",
    )?;
    let rows = stmt.query_map(params![tool_result_message_id], row_to_draft_version)?;
    let versions = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use tempfile::tempdir;

    fn open_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let dir = tempdir().unwrap();
        let key = crate::crypto::generate_key();
        let pool = init_db(&dir.path().join("test.db"), &key).unwrap();
        (dir, pool)
    }

    #[test]
    fn test_create_and_get_session() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        let s = create_chat_session(&conn, "global", None, "My Chat").unwrap();
        assert_eq!(s.scope, "global");
        assert_eq!(s.title, "My Chat");
        let s2 = get_chat_session(&conn, &s.id).unwrap();
        assert_eq!(s.id, s2.id);
    }

    #[test]
    fn test_get_or_create_patient_session_idempotent() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        // Create a patient first
        conn.execute(
            "INSERT INTO patients (id, first_name, last_name, date_of_birth, ahv_number)
             VALUES ('p1', 'Anna', 'Muster', '1980-01-01', '756.1234.5678.97')",
            [],
        )
        .unwrap();
        let s1 = get_or_create_patient_session(&conn, "p1").unwrap();
        let s2 = get_or_create_patient_session(&conn, "p1").unwrap();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn test_append_and_list_messages() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        let s = create_chat_session(&conn, "global", None, "T").unwrap();
        append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: s.id.clone(),
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_name: None,
                tool_args_json: None,
                tool_result_for: None,
            },
        )
        .unwrap();
        append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: s.id.clone(),
                role: "assistant".to_string(),
                content: "Hi there".to_string(),
                tool_name: None,
                tool_args_json: None,
                tool_result_for: None,
            },
        )
        .unwrap();
        let msgs = list_chat_messages(&conn, &s.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn test_draft_versions_are_ordered_and_reversible() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        let session = create_chat_session(&conn, "global", None, "T").unwrap();
        let tool_result = append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: session.id,
                role: "tool_result".to_string(),
                content: "{}".to_string(),
                tool_name: Some("write_report".to_string()),
                tool_args_json: None,
                tool_result_for: None,
            },
        )
        .unwrap();
        let first = append_chat_draft_version(
            &conn,
            &CreateChatDraftVersion {
                tool_result_message_id: tool_result.id.clone(),
                content: "AI draft".to_string(),
                origin: "ai".to_string(),
                claim_resolutions_json: "[]".to_string(),
            },
        )
        .unwrap();
        let second = append_chat_draft_version(
            &conn,
            &CreateChatDraftVersion {
                tool_result_message_id: tool_result.id.clone(),
                content: "Clinician revision".to_string(),
                origin: "manual".to_string(),
                claim_resolutions_json:
                    r#"[{"id":"missing-typed-source","resolution":"uncertain"}]"#.to_string(),
            },
        )
        .unwrap();
        let versions = list_chat_draft_versions(&conn, &tool_result.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].id, first.id);
        assert_eq!(versions[0].version_number, 1);
        assert_eq!(versions[1].id, second.id);
        assert_eq!(versions[1].content, "Clinician revision");
    }

    #[test]
    fn test_delete_session_cascades() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        let s = create_chat_session(&conn, "global", None, "T").unwrap();
        append_chat_message(
            &conn,
            &CreateChatMessage {
                session_id: s.id.clone(),
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_name: None,
                tool_args_json: None,
                tool_result_for: None,
            },
        )
        .unwrap();
        delete_chat_session(&conn, &s.id).unwrap();
        let msgs = list_chat_messages(&conn, &s.id).unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_rename_session() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        let s = create_chat_session(&conn, "global", None, "Old").unwrap();
        let s2 = update_chat_session_title(&conn, &s.id, "New").unwrap();
        assert_eq!(s2.title, "New");
    }
}

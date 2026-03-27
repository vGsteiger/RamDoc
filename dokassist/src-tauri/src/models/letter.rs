use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Letter {
    pub id: String,
    pub patient_id: String,
    pub letter_type: String,
    pub template_language: String,
    pub recipient_name: Option<String>,
    pub recipient_address: Option<String>,
    pub subject: String,
    pub content: String,
    pub status: String,
    pub model_name: Option<String>,
    pub session_ids: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub finalized_at: Option<String>,
    pub sent_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLetter {
    pub patient_id: String,
    pub letter_type: String,
    pub template_language: String,
    pub recipient_name: Option<String>,
    pub recipient_address: Option<String>,
    pub subject: String,
    pub content: String,
    pub model_name: Option<String>,
    pub session_ids: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLetter {
    pub letter_type: Option<String>,
    pub template_language: Option<String>,
    pub recipient_name: Option<String>,
    pub recipient_address: Option<String>,
    pub subject: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub model_name: Option<String>,
    pub session_ids: Option<String>,
}

fn row_to_letter(row: &Row) -> Result<Letter, rusqlite::Error> {
    Ok(Letter {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        letter_type: row.get(2)?,
        template_language: row.get(3)?,
        recipient_name: row.get(4)?,
        recipient_address: row.get(5)?,
        subject: row.get(6)?,
        content: row.get(7)?,
        status: row.get(8)?,
        model_name: row.get(9)?,
        session_ids: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        finalized_at: row.get(13)?,
        sent_at: row.get(14)?,
    })
}

pub fn create_letter(conn: &Connection, input: CreateLetter) -> Result<Letter, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO letters (id, patient_id, letter_type, template_language, recipient_name, recipient_address, subject, content, model_name, session_ids)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.letter_type,
            input.template_language,
            input.recipient_name,
            input.recipient_address,
            input.subject,
            input.content,
            input.model_name,
            input.session_ids,
        ],
    )?;

    get_letter(conn, &id)
}

pub fn get_letter(conn: &Connection, id: &str) -> Result<Letter, AppError> {
    let letter = conn
        .query_row(
            "SELECT id, patient_id, letter_type, template_language, recipient_name, recipient_address, subject, content, status, model_name, session_ids, created_at, updated_at, finalized_at, sent_at
             FROM letters WHERE id = ?",
            params![id],
            row_to_letter,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Letter not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(letter)
}

pub fn update_letter(conn: &Connection, id: &str, input: UpdateLetter) -> Result<Letter, AppError> {
    get_letter(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(letter_type) = input.letter_type {
        updates.push("letter_type = ?");
        values.push(Box::new(letter_type));
    }
    if let Some(template_language) = input.template_language {
        updates.push("template_language = ?");
        values.push(Box::new(template_language));
    }
    if let Some(recipient_name) = input.recipient_name {
        updates.push("recipient_name = ?");
        values.push(Box::new(recipient_name));
    }
    if let Some(recipient_address) = input.recipient_address {
        updates.push("recipient_address = ?");
        values.push(Box::new(recipient_address));
    }
    if let Some(subject) = input.subject {
        updates.push("subject = ?");
        values.push(Box::new(subject));
    }
    if let Some(content) = input.content {
        updates.push("content = ?");
        values.push(Box::new(content));
    }
    if let Some(status) = input.status {
        updates.push("status = ?");
        values.push(Box::new(status));
    }
    if let Some(model_name) = input.model_name {
        updates.push("model_name = ?");
        values.push(Box::new(model_name));
    }
    if let Some(session_ids) = input.session_ids {
        updates.push("session_ids = ?");
        values.push(Box::new(session_ids));
    }

    if updates.is_empty() {
        return get_letter(conn, id);
    }

    let query = format!("UPDATE letters SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_letter(conn, id)
}

pub fn delete_letter(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM letters WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Letter not found: {}", id)));
    }

    Ok(())
}

pub fn list_letters_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Letter>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, letter_type, template_language, recipient_name, recipient_address, subject, content, status, model_name, session_ids, created_at, updated_at, finalized_at, sent_at
         FROM letters
         WHERE patient_id = ?
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )?;

    let letters = stmt
        .query_map(params![patient_id, limit, offset], row_to_letter)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(letters)
}

pub fn mark_letter_as_finalized(conn: &Connection, id: &str) -> Result<Letter, AppError> {
    get_letter(conn, id)?;

    conn.execute(
        "UPDATE letters SET status = 'finalized', finalized_at = datetime('now') WHERE id = ?",
        params![id],
    )?;

    get_letter(conn, id)
}

pub fn mark_letter_as_sent(conn: &Connection, id: &str) -> Result<Letter, AppError> {
    get_letter(conn, id)?;

    conn.execute(
        "UPDATE letters SET status = 'sent', sent_at = datetime('now') WHERE id = ?",
        params![id],
    )?;

    get_letter(conn, id)
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

    fn insert_patient(conn: &Connection) {
        conn.execute(
            "INSERT INTO patients (id, first_name, last_name, date_of_birth, ahv_number)
             VALUES ('p1', 'Anna', 'Test', '1985-01-01', '756.1234.5678.97')",
            [],
        )
        .unwrap();
    }

    fn make_letter(conn: &Connection) -> Letter {
        create_letter(
            conn,
            CreateLetter {
                patient_id: "p1".into(),
                letter_type: "referral".into(),
                template_language: "de".into(),
                recipient_name: Some("Dr. Müller".into()),
                recipient_address: Some("Bahnhofstrasse 1, 8001 Zürich".into()),
                subject: "Zuweisung Patient Anna Test".into(),
                content: "Sehr geehrter Herr Dr. Müller, ...".into(),
                model_name: Some("llama-3.2-3b".into()),
                session_ids: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_and_get_letter() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let letter = make_letter(&conn);
        assert_eq!(letter.status, "draft");
        assert_eq!(letter.letter_type, "referral");
        assert_eq!(letter.template_language, "de");
        assert!(letter.sent_at.is_none());
        let letter2 = get_letter(&conn, &letter.id).unwrap();
        assert_eq!(letter.id, letter2.id);
    }

    #[test]
    fn test_update_letter_content() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let letter = make_letter(&conn);
        let updated = update_letter(
            &conn,
            &letter.id,
            UpdateLetter {
                letter_type: None,
                template_language: None,
                recipient_name: None,
                recipient_address: None,
                subject: None,
                content: Some("Updated content.".into()),
                status: None,
                model_name: None,
                session_ids: None,
            },
        )
        .unwrap();
        assert_eq!(updated.content, "Updated content.");
        assert_eq!(updated.letter_type, "referral");
    }

    #[test]
    fn test_delete_letter() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let letter = make_letter(&conn);
        delete_letter(&conn, &letter.id).unwrap();
        assert!(matches!(
            get_letter(&conn, &letter.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_letters_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        make_letter(&conn);
        make_letter(&conn);
        let list = list_letters_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_mark_letter_as_finalized() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let letter = make_letter(&conn);
        assert_eq!(letter.status, "draft");
        let finalized = mark_letter_as_finalized(&conn, &letter.id).unwrap();
        assert_eq!(finalized.status, "finalized");
        assert!(finalized.finalized_at.is_some());
    }

    #[test]
    fn test_mark_letter_as_sent() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let letter = make_letter(&conn);
        assert_eq!(letter.status, "draft");
        let sent = mark_letter_as_sent(&conn, &letter.id).unwrap();
        assert_eq!(sent.status, "sent");
        assert!(sent.sent_at.is_some());
    }

    #[test]
    fn test_mark_nonexistent_letter_as_sent() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        assert!(matches!(
            mark_letter_as_sent(&conn, "nonexistent-id"),
            Err(AppError::NotFound(_))
        ));
    }
}

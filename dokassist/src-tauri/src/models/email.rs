use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub patient_id: String,
    pub recipient_email: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub sent_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmail {
    pub patient_id: String,
    pub recipient_email: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEmail {
    pub recipient_email: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub status: Option<String>,
}

fn row_to_email(row: &Row) -> Result<Email, rusqlite::Error> {
    Ok(Email {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        recipient_email: row.get(2)?,
        subject: row.get(3)?,
        body: row.get(4)?,
        status: row.get(5)?,
        sent_at: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn create_email(conn: &Connection, input: CreateEmail) -> Result<Email, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO emails (id, patient_id, recipient_email, subject, body)
         VALUES (?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.recipient_email,
            input.subject,
            input.body,
        ],
    )?;

    get_email(conn, &id)
}

pub fn get_email(conn: &Connection, id: &str) -> Result<Email, AppError> {
    let email = conn
        .query_row(
            "SELECT id, patient_id, recipient_email, subject, body, status, sent_at, created_at, updated_at
             FROM emails WHERE id = ?",
            params![id],
            row_to_email,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Email not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(email)
}

pub fn update_email(conn: &Connection, id: &str, input: UpdateEmail) -> Result<Email, AppError> {
    get_email(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(recipient_email) = input.recipient_email {
        updates.push("recipient_email = ?");
        values.push(Box::new(recipient_email));
    }
    if let Some(subject) = input.subject {
        updates.push("subject = ?");
        values.push(Box::new(subject));
    }
    if let Some(body) = input.body {
        updates.push("body = ?");
        values.push(Box::new(body));
    }
    if let Some(status) = input.status {
        updates.push("status = ?");
        values.push(Box::new(status));
    }

    if updates.is_empty() {
        return get_email(conn, id);
    }

    let query = format!("UPDATE emails SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_email(conn, id)
}

pub fn delete_email(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM emails WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Email not found: {}", id)));
    }

    Ok(())
}

pub fn list_emails_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Email>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, recipient_email, subject, body, status, sent_at, created_at, updated_at
         FROM emails
         WHERE patient_id = ?
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )?;

    let emails = stmt
        .query_map(params![patient_id, limit, offset], row_to_email)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(emails)
}

pub fn mark_email_as_sent(conn: &Connection, id: &str) -> Result<Email, AppError> {
    get_email(conn, id)?;

    conn.execute(
        "UPDATE emails SET status = 'sent', sent_at = datetime('now') WHERE id = ?",
        params![id],
    )?;

    get_email(conn, id)
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

    fn make_email(conn: &Connection) -> Email {
        create_email(
            conn,
            CreateEmail {
                patient_id: "p1".into(),
                recipient_email: "doc@example.com".into(),
                subject: "Patient Update".into(),
                body: "Dear Doctor, ...".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_and_get_email() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let e = make_email(&conn);
        assert_eq!(e.status, "draft");
        assert_eq!(e.subject, "Patient Update");
        assert!(e.sent_at.is_none());
        let e2 = get_email(&conn, &e.id).unwrap();
        assert_eq!(e.id, e2.id);
    }

    #[test]
    fn test_update_email_subject() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let e = make_email(&conn);
        let updated = update_email(
            &conn,
            &e.id,
            UpdateEmail {
                recipient_email: None,
                subject: Some("Updated Subject".into()),
                body: None,
                status: None,
            },
        )
        .unwrap();
        assert_eq!(updated.subject, "Updated Subject");
        assert_eq!(updated.body, "Dear Doctor, ...");
    }

    #[test]
    fn test_delete_email() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let e = make_email(&conn);
        delete_email(&conn, &e.id).unwrap();
        assert!(matches!(
            get_email(&conn, &e.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_emails_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        make_email(&conn);
        make_email(&conn);
        let list = list_emails_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_mark_email_as_sent() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let e = make_email(&conn);
        assert_eq!(e.status, "draft");
        let sent = mark_email_as_sent(&conn, &e.id).unwrap();
        assert_eq!(sent.status, "sent");
        assert!(sent.sent_at.is_some());
    }

    #[test]
    fn test_mark_nonexistent_email_as_sent() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        assert!(matches!(
            mark_email_as_sent(&conn, "nonexistent-id"),
            Err(AppError::NotFound(_))
        ));
    }
}

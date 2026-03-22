use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub id: String,
    pub patient_id: String,
    pub icd10_code: String,
    pub description: String,
    pub status: String,
    pub diagnosed_date: String,
    pub resolved_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiagnosis {
    pub patient_id: String,
    pub icd10_code: String,
    pub description: String,
    pub status: Option<String>,
    pub diagnosed_date: String,
    pub resolved_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiagnosis {
    pub icd10_code: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub diagnosed_date: Option<String>,
    pub resolved_date: Option<String>,
    pub notes: Option<String>,
}

fn row_to_diagnosis(row: &Row) -> Result<Diagnosis, rusqlite::Error> {
    Ok(Diagnosis {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        icd10_code: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        diagnosed_date: row.get(5)?,
        resolved_date: row.get(6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn create_diagnosis(conn: &Connection, input: CreateDiagnosis) -> Result<Diagnosis, AppError> {
    let id = Uuid::now_v7().to_string();
    let status = input.status.unwrap_or_else(|| "active".to_string());

    conn.execute(
        "INSERT INTO diagnoses (id, patient_id, icd10_code, description, status, diagnosed_date, resolved_date, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.icd10_code,
            input.description,
            status,
            input.diagnosed_date,
            input.resolved_date,
            input.notes,
        ],
    )?;

    get_diagnosis(conn, &id)
}

pub fn get_diagnosis(conn: &Connection, id: &str) -> Result<Diagnosis, AppError> {
    let diagnosis = conn
        .query_row(
            "SELECT id, patient_id, icd10_code, description, status, diagnosed_date, resolved_date, notes,
                    created_at, updated_at
             FROM diagnoses WHERE id = ?",
            params![id],
            row_to_diagnosis,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Diagnosis not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(diagnosis)
}

pub fn update_diagnosis(
    conn: &Connection,
    id: &str,
    input: UpdateDiagnosis,
) -> Result<Diagnosis, AppError> {
    get_diagnosis(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(icd10_code) = input.icd10_code {
        updates.push("icd10_code = ?");
        values.push(Box::new(icd10_code));
    }
    if let Some(description) = input.description {
        updates.push("description = ?");
        values.push(Box::new(description));
    }
    if let Some(status) = input.status {
        updates.push("status = ?");
        values.push(Box::new(status));
    }
    if let Some(diagnosed_date) = input.diagnosed_date {
        updates.push("diagnosed_date = ?");
        values.push(Box::new(diagnosed_date));
    }
    if let Some(resolved_date) = input.resolved_date {
        updates.push("resolved_date = ?");
        values.push(Box::new(resolved_date));
    }
    if let Some(notes) = input.notes {
        updates.push("notes = ?");
        values.push(Box::new(notes));
    }

    if updates.is_empty() {
        return get_diagnosis(conn, id);
    }

    let query = format!("UPDATE diagnoses SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_diagnosis(conn, id)
}

pub fn delete_diagnosis(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM diagnoses WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Diagnosis not found: {}", id)));
    }

    Ok(())
}

pub fn list_diagnoses_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Diagnosis>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, icd10_code, description, status, diagnosed_date, resolved_date, notes,
                created_at, updated_at
         FROM diagnoses
         WHERE patient_id = ?
         ORDER BY diagnosed_date DESC
         LIMIT ? OFFSET ?",
    )?;

    let diagnoses = stmt
        .query_map(params![patient_id, limit, offset], row_to_diagnosis)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(diagnoses)
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

    #[test]
    fn test_create_and_get_diagnosis() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let d = create_diagnosis(
            &conn,
            CreateDiagnosis {
                patient_id: "p1".into(),
                icd10_code: "F32.1".into(),
                description: "Moderate depressive episode".into(),
                status: Some("active".into()),
                diagnosed_date: "2026-01-15".into(),
                resolved_date: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(d.icd10_code, "F32.1");
        assert_eq!(d.patient_id, "p1");
        let d2 = get_diagnosis(&conn, &d.id).unwrap();
        assert_eq!(d.id, d2.id);
        assert_eq!(d2.description, "Moderate depressive episode");
    }

    #[test]
    fn test_create_diagnosis_default_status() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let d = create_diagnosis(
            &conn,
            CreateDiagnosis {
                patient_id: "p1".into(),
                icd10_code: "F41.0".into(),
                description: "Panic disorder".into(),
                status: None,
                diagnosed_date: "2026-02-01".into(),
                resolved_date: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(d.status, "active");
    }

    #[test]
    fn test_update_diagnosis() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let d = create_diagnosis(
            &conn,
            CreateDiagnosis {
                patient_id: "p1".into(),
                icd10_code: "F32.0".into(),
                description: "Mild depressive episode".into(),
                status: None,
                diagnosed_date: "2026-01-01".into(),
                resolved_date: None,
                notes: None,
            },
        )
        .unwrap();
        let updated = update_diagnosis(
            &conn,
            &d.id,
            UpdateDiagnosis {
                icd10_code: Some("F32.1".into()),
                description: None,
                status: None,
                diagnosed_date: None,
                resolved_date: Some("2026-03-01".into()),
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(updated.icd10_code, "F32.1");
        assert_eq!(updated.description, "Mild depressive episode");
        assert_eq!(updated.resolved_date, Some("2026-03-01".into()));
    }

    #[test]
    fn test_delete_diagnosis() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let d = create_diagnosis(
            &conn,
            CreateDiagnosis {
                patient_id: "p1".into(),
                icd10_code: "F40.1".into(),
                description: "Social phobia".into(),
                status: None,
                diagnosed_date: "2026-01-01".into(),
                resolved_date: None,
                notes: None,
            },
        )
        .unwrap();
        delete_diagnosis(&conn, &d.id).unwrap();
        assert!(matches!(
            get_diagnosis(&conn, &d.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_diagnoses_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        for (code, date) in [
            ("F32.0", "2026-01-01"),
            ("F41.0", "2026-02-01"),
            ("F40.0", "2026-03-01"),
        ] {
            create_diagnosis(
                &conn,
                CreateDiagnosis {
                    patient_id: "p1".into(),
                    icd10_code: code.into(),
                    description: code.into(),
                    status: None,
                    diagnosed_date: date.into(),
                    resolved_date: None,
                    notes: None,
                },
            )
            .unwrap();
        }
        let list = list_diagnoses_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].diagnosed_date, "2026-03-01");
        assert_eq!(list[2].diagnosed_date, "2026-01-01");
    }
}

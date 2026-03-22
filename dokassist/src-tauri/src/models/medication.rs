use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Medication {
    pub id: String,
    pub patient_id: String,
    pub substance: String,
    pub dosage: String,
    pub frequency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMedication {
    pub patient_id: String,
    pub substance: String,
    pub dosage: String,
    pub frequency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMedication {
    pub substance: Option<String>,
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

fn row_to_medication(row: &Row) -> Result<Medication, rusqlite::Error> {
    Ok(Medication {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        substance: row.get(2)?,
        dosage: row.get(3)?,
        frequency: row.get(4)?,
        start_date: row.get(5)?,
        end_date: row.get(6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn create_medication(
    conn: &Connection,
    input: CreateMedication,
) -> Result<Medication, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO medications (id, patient_id, substance, dosage, frequency, start_date, end_date, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.substance,
            input.dosage,
            input.frequency,
            input.start_date,
            input.end_date,
            input.notes,
        ],
    )?;

    get_medication(conn, &id)
}

pub fn get_medication(conn: &Connection, id: &str) -> Result<Medication, AppError> {
    let medication = conn
        .query_row(
            "SELECT id, patient_id, substance, dosage, frequency, start_date, end_date, notes,
                    created_at, updated_at
             FROM medications WHERE id = ?",
            params![id],
            row_to_medication,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Medication not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(medication)
}

pub fn update_medication(
    conn: &Connection,
    id: &str,
    input: UpdateMedication,
) -> Result<Medication, AppError> {
    get_medication(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(substance) = input.substance {
        updates.push("substance = ?");
        values.push(Box::new(substance));
    }
    if let Some(dosage) = input.dosage {
        updates.push("dosage = ?");
        values.push(Box::new(dosage));
    }
    if let Some(frequency) = input.frequency {
        updates.push("frequency = ?");
        values.push(Box::new(frequency));
    }
    if let Some(start_date) = input.start_date {
        updates.push("start_date = ?");
        values.push(Box::new(start_date));
    }
    if let Some(end_date) = input.end_date {
        updates.push("end_date = ?");
        values.push(Box::new(end_date));
    }
    if let Some(notes) = input.notes {
        updates.push("notes = ?");
        values.push(Box::new(notes));
    }

    if updates.is_empty() {
        return get_medication(conn, id);
    }

    let query = format!("UPDATE medications SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_medication(conn, id)
}

pub fn delete_medication(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM medications WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Medication not found: {}", id)));
    }

    Ok(())
}

pub fn list_medications_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Medication>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, substance, dosage, frequency, start_date, end_date, notes,
                created_at, updated_at
         FROM medications
         WHERE patient_id = ?
         ORDER BY start_date DESC
         LIMIT ? OFFSET ?",
    )?;

    let medications = stmt
        .query_map(params![patient_id, limit, offset], row_to_medication)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(medications)
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
    fn test_create_and_get_medication() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let m = create_medication(
            &conn,
            CreateMedication {
                patient_id: "p1".into(),
                substance: "Sertraline".into(),
                dosage: "50mg".into(),
                frequency: "daily".into(),
                start_date: "2026-01-01".into(),
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(m.substance, "Sertraline");
        assert_eq!(m.dosage, "50mg");
        assert_eq!(m.frequency, "daily");
        let m2 = get_medication(&conn, &m.id).unwrap();
        assert_eq!(m.id, m2.id);
    }

    #[test]
    fn test_update_medication() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let m = create_medication(
            &conn,
            CreateMedication {
                patient_id: "p1".into(),
                substance: "Fluoxetine".into(),
                dosage: "20mg".into(),
                frequency: "daily".into(),
                start_date: "2026-01-01".into(),
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        let updated = update_medication(
            &conn,
            &m.id,
            UpdateMedication {
                substance: None,
                dosage: Some("40mg".into()),
                frequency: None,
                start_date: None,
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(updated.dosage, "40mg");
        assert_eq!(updated.substance, "Fluoxetine");
    }

    #[test]
    fn test_delete_medication() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let m = create_medication(
            &conn,
            CreateMedication {
                patient_id: "p1".into(),
                substance: "Lorazepam".into(),
                dosage: "1mg".into(),
                frequency: "as needed".into(),
                start_date: "2026-01-01".into(),
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        delete_medication(&conn, &m.id).unwrap();
        assert!(matches!(
            get_medication(&conn, &m.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_medications_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        for (substance, date) in [
            ("Med A", "2026-01-01"),
            ("Med B", "2026-02-01"),
            ("Med C", "2026-03-01"),
        ] {
            create_medication(
                &conn,
                CreateMedication {
                    patient_id: "p1".into(),
                    substance: substance.into(),
                    dosage: "10mg".into(),
                    frequency: "daily".into(),
                    start_date: date.into(),
                    end_date: None,
                    notes: None,
                },
            )
            .unwrap();
        }
        let list = list_medications_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].start_date, "2026-03-01");
        assert_eq!(list[2].start_date, "2026-01-01");
    }

    #[test]
    fn test_update_medication_no_fields() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let m = create_medication(
            &conn,
            CreateMedication {
                patient_id: "p1".into(),
                substance: "Quetiapine".into(),
                dosage: "25mg".into(),
                frequency: "nightly".into(),
                start_date: "2026-01-01".into(),
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        let unchanged = update_medication(
            &conn,
            &m.id,
            UpdateMedication {
                substance: None,
                dosage: None,
                frequency: None,
                start_date: None,
                end_date: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(unchanged.substance, "Quetiapine");
        assert_eq!(unchanged.dosage, "25mg");
    }
}

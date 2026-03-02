use crate::ahv::validate_ahv;
use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: String,
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePatient {
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePatient {
    pub ahv_number: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
}

fn row_to_patient(row: &Row) -> Result<Patient, rusqlite::Error> {
    Ok(Patient {
        id: row.get(0)?,
        ahv_number: row.get(1)?,
        first_name: row.get(2)?,
        last_name: row.get(3)?,
        date_of_birth: row.get(4)?,
        gender: row.get(5)?,
        address: row.get(6)?,
        phone: row.get(7)?,
        email: row.get(8)?,
        insurance: row.get(9)?,
        gp_name: row.get(10)?,
        gp_address: row.get(11)?,
        notes: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

pub fn create_patient(conn: &Connection, input: CreatePatient) -> Result<Patient, AppError> {
    // Validate and normalize AHV number
    let normalized_ahv = validate_ahv(&input.ahv_number)?;

    // Generate UUIDv7 (time-sortable)
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO patients (
            id, ahv_number, first_name, last_name, date_of_birth,
            gender, address, phone, email, insurance, gp_name, gp_address, notes
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            normalized_ahv,
            input.first_name,
            input.last_name,
            input.date_of_birth,
            input.gender,
            input.address,
            input.phone,
            input.email,
            input.insurance,
            input.gp_name,
            input.gp_address,
            input.notes,
        ],
    )?;

    get_patient(conn, &id)
}

pub fn get_patient(conn: &Connection, id: &str) -> Result<Patient, AppError> {
    let patient = conn
        .query_row(
            "SELECT id, ahv_number, first_name, last_name, date_of_birth,
                    gender, address, phone, email, insurance, gp_name, gp_address, notes,
                    created_at, updated_at
             FROM patients WHERE id = ?",
            params![id],
            row_to_patient,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Patient not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(patient)
}

pub fn update_patient(
    conn: &Connection,
    id: &str,
    input: UpdatePatient,
) -> Result<Patient, AppError> {
    // Check if patient exists
    get_patient(conn, id)?;

    // Validate AHV if provided
    let normalized_ahv = if let Some(ref ahv) = input.ahv_number {
        Some(validate_ahv(ahv)?)
    } else {
        None
    };

    // Build dynamic UPDATE query based on provided fields
    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ahv) = normalized_ahv {
        updates.push("ahv_number = ?");
        values.push(Box::new(ahv));
    }
    if let Some(first_name) = input.first_name {
        updates.push("first_name = ?");
        values.push(Box::new(first_name));
    }
    if let Some(last_name) = input.last_name {
        updates.push("last_name = ?");
        values.push(Box::new(last_name));
    }
    if let Some(date_of_birth) = input.date_of_birth {
        updates.push("date_of_birth = ?");
        values.push(Box::new(date_of_birth));
    }
    if let Some(gender) = input.gender {
        updates.push("gender = ?");
        values.push(Box::new(gender));
    }
    if let Some(address) = input.address {
        updates.push("address = ?");
        values.push(Box::new(address));
    }
    if let Some(phone) = input.phone {
        updates.push("phone = ?");
        values.push(Box::new(phone));
    }
    if let Some(email) = input.email {
        updates.push("email = ?");
        values.push(Box::new(email));
    }
    if let Some(insurance) = input.insurance {
        updates.push("insurance = ?");
        values.push(Box::new(insurance));
    }
    if let Some(gp_name) = input.gp_name {
        updates.push("gp_name = ?");
        values.push(Box::new(gp_name));
    }
    if let Some(gp_address) = input.gp_address {
        updates.push("gp_address = ?");
        values.push(Box::new(gp_address));
    }
    if let Some(notes) = input.notes {
        updates.push("notes = ?");
        values.push(Box::new(notes));
    }

    if updates.is_empty() {
        return get_patient(conn, id);
    }

    let query = format!("UPDATE patients SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_patient(conn, id)
}

pub fn delete_patient(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM patients WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Patient not found: {}", id)));
    }

    Ok(())
}

pub fn list_patients(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<Patient>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, ahv_number, first_name, last_name, date_of_birth,
                gender, address, phone, email, insurance, gp_name, gp_address, notes,
                created_at, updated_at
         FROM patients
         ORDER BY last_name, first_name
         LIMIT ? OFFSET ?",
    )?;

    let patients = stmt
        .query_map(params![limit, offset], row_to_patient)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(patients)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        let pool = init_db(&db_path, &key).unwrap();
        let conn = pool.conn().unwrap();
        // Return both dir and connection (dir must stay alive)
        // We need to extract the connection from the MutexGuard
        // For testing, let's create a new connection directly
        let conn = Connection::open(&db_path).unwrap();
        let key_hex = hex::encode(&key);
        conn.execute(&format!("PRAGMA key = \"x'{}'\";", key_hex), [])
            .unwrap();
        (dir, conn)
    }

    // Helper function to generate valid AHV numbers for testing
    fn generate_test_ahv(index: usize) -> String {
        // Base: 756 + 8 digits + checksum
        let base = format!("75600000{:04}", index);

        // Calculate EAN-13 checksum
        let sum: u32 = base
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let digit = c.to_digit(10).unwrap();
                if i % 2 == 0 {
                    digit
                } else {
                    digit * 3
                }
            })
            .sum();

        let checksum = (10 - (sum % 10)) % 10;
        format!("{}{}", base, checksum)
    }

    #[test]
    fn test_create_and_get_patient() {
        let (_dir, conn) = setup_test_db();

        let input = CreatePatient {
            ahv_number: "7561234567897".to_string(),
            first_name: "Hans".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-15".to_string(),
            gender: Some("male".to_string()),
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };

        let patient = create_patient(&conn, input).unwrap();
        assert_eq!(patient.ahv_number, "756.1234.5678.97");
        assert_eq!(patient.first_name, "Hans");
        assert_eq!(patient.last_name, "Müller");

        let retrieved = get_patient(&conn, &patient.id).unwrap();
        assert_eq!(retrieved.id, patient.id);
    }

    #[test]
    fn test_update_patient() {
        let (_dir, conn) = setup_test_db();

        let input = CreatePatient {
            ahv_number: "7561234567897".to_string(),
            first_name: "Hans".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-15".to_string(),
            gender: Some("male".to_string()),
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };

        let patient = create_patient(&conn, input).unwrap();

        let update = UpdatePatient {
            first_name: Some("Peter".to_string()),
            phone: Some("+41791234567".to_string()),
            ..Default::default()
        };

        let updated = update_patient(&conn, &patient.id, update).unwrap();
        assert_eq!(updated.first_name, "Peter");
        assert_eq!(updated.phone, Some("+41791234567".to_string()));
        assert_eq!(updated.last_name, "Müller"); // unchanged
    }

    #[test]
    fn test_delete_patient() {
        let (_dir, conn) = setup_test_db();

        let input = CreatePatient {
            ahv_number: "7561234567897".to_string(),
            first_name: "Hans".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-15".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };

        let patient = create_patient(&conn, input).unwrap();
        delete_patient(&conn, &patient.id).unwrap();

        let result = get_patient(&conn, &patient.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_patients() {
        let (_dir, conn) = setup_test_db();

        // Create multiple patients with valid AHV numbers
        for i in 0..5 {
            let input = CreatePatient {
                ahv_number: generate_test_ahv(i),
                first_name: format!("Test{}", i),
                last_name: format!("User{}", i),
                date_of_birth: "1980-01-01".to_string(),
                gender: None,
                address: None,
                phone: None,
                email: None,
                insurance: None,
                gp_name: None,
                gp_address: None,
                notes: None,
            };
            create_patient(&conn, input).unwrap();
        }

        let patients = list_patients(&conn, 10, 0).unwrap();
        assert_eq!(patients.len(), 5);
    }

    // ========== SQL Injection Security Tests ==========

    #[test]
    fn test_sql_injection_in_create_first_name() {
        let (_dir, conn) = setup_test_db();

        // Attempt SQL injection via first_name field
        let malicious_input = CreatePatient {
            ahv_number: generate_test_ahv(999),
            first_name: "'; DROP TABLE patients; --".to_string(),
            last_name: "TestUser".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };

        let patient = create_patient(&conn, malicious_input).unwrap();

        // Verify the malicious string was stored as literal data
        assert_eq!(patient.first_name, "'; DROP TABLE patients; --");

        // Verify table still exists by querying it
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_sql_injection_in_update_phone() {
        let (_dir, conn) = setup_test_db();

        // Create a patient first
        let input = CreatePatient {
            ahv_number: generate_test_ahv(100),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: None,
            address: None,
            phone: Some("+41791234567".to_string()),
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };
        let patient = create_patient(&conn, input).unwrap();

        // Attempt SQL injection via phone field update
        let malicious_update = UpdatePatient {
            phone: Some("'; DROP TABLE patients; --".to_string()),
            ..Default::default()
        };

        let result = update_patient(&conn, &patient.id, malicious_update);
        assert!(result.is_ok());

        // Verify the malicious string was stored as literal data
        let updated = get_patient(&conn, &patient.id).unwrap();
        assert_eq!(updated.phone.unwrap(), "'; DROP TABLE patients; --");

        // Verify table still exists (not dropped)
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_sql_injection_in_update_multiple_fields() {
        let (_dir, conn) = setup_test_db();

        // Create a patient
        let input = CreatePatient {
            ahv_number: generate_test_ahv(101),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };
        let patient = create_patient(&conn, input).unwrap();

        // Attempt SQL injection in multiple fields
        let malicious_update = UpdatePatient {
            email: Some("admin@test.com' OR '1'='1".to_string()),
            notes: Some("'; DELETE FROM patients WHERE '1'='1".to_string()),
            address: Some(
                "123 Main St'; UPDATE patients SET ahv_number='000' WHERE '1'='1".to_string(),
            ),
            ..Default::default()
        };

        let result = update_patient(&conn, &patient.id, malicious_update);
        assert!(result.is_ok());

        // Verify all malicious strings were stored as literal data
        let updated = get_patient(&conn, &patient.id).unwrap();
        assert_eq!(updated.email.unwrap(), "admin@test.com' OR '1'='1");
        assert_eq!(
            updated.notes.unwrap(),
            "'; DELETE FROM patients WHERE '1'='1"
        );
        assert_eq!(
            updated.address.unwrap(),
            "123 Main St'; UPDATE patients SET ahv_number='000' WHERE '1'='1"
        );

        // Verify patient data unchanged (AHV still original)
        assert_eq!(updated.ahv_number, "756.0000.0101.53"); // Original AHV
    }

    #[test]
    fn test_sql_injection_in_notes_field() {
        let (_dir, conn) = setup_test_db();

        // Attempt SQL injection via notes field (common place for free-text)
        let malicious_input = CreatePatient {
            ahv_number: generate_test_ahv(102),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: Some(
                "Patient fine. '; DROP TABLE sessions; DROP TABLE diagnoses; --".to_string(),
            ),
        };

        let patient = create_patient(&conn, malicious_input).unwrap();

        // Verify the malicious string was stored as literal data
        assert_eq!(
            patient.notes.unwrap(),
            "Patient fine. '; DROP TABLE sessions; DROP TABLE diagnoses; --"
        );

        // Verify related tables still exist
        conn.execute("SELECT COUNT(*) FROM sessions", []).unwrap();
        conn.execute("SELECT COUNT(*) FROM diagnoses", []).unwrap();
    }

    #[test]
    fn test_sql_injection_classic_or_bypass() {
        let (_dir, conn) = setup_test_db();

        // Create a patient
        let input = CreatePatient {
            ahv_number: generate_test_ahv(103),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        };
        let patient = create_patient(&conn, input).unwrap();

        // Attempt classic OR '1'='1 injection in update
        let malicious_update = UpdatePatient {
            last_name: Some("Smith' OR '1'='1".to_string()),
            ..Default::default()
        };

        update_patient(&conn, &patient.id, malicious_update).unwrap();

        // Verify only ONE patient was updated (not all due to OR condition)
        let patients = list_patients(&conn, 100, 0).unwrap();
        assert_eq!(patients.len(), 1);
        assert_eq!(patients[0].last_name, "Smith' OR '1'='1"); // Stored as literal
    }

    #[test]
    fn test_sql_injection_unicode_and_special_chars() {
        let (_dir, conn) = setup_test_db();

        // Test with various special characters and unicode
        let malicious_input = CreatePatient {
            ahv_number: generate_test_ahv(104),
            first_name: "Hans\"; DROP TABLE patients; --".to_string(),
            last_name: "Müller' OR 1=1; --".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: Some("male\x00admin".to_string()), // Null byte injection attempt
            address: Some("Straße 123\'; DELETE FROM patients WHERE \'x\'=\'x".to_string()),
            phone: None,
            email: Some(
                "test@test.com\"; UPDATE patients SET notes='hacked' WHERE \"\"=\"".to_string(),
            ),
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: Some("普通話'; DROP TABLE patients; --".to_string()), // Unicode with injection
        };

        let patient = create_patient(&conn, malicious_input).unwrap();

        // Verify all strings stored as literal data
        assert!(patient.first_name.contains("DROP TABLE"));
        assert!(patient.last_name.contains("OR 1=1"));
        assert!(patient.email.unwrap().contains("UPDATE patients"));
        assert!(patient.notes.unwrap().contains("普通話"));

        // Verify table still exists and intact
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM patients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}

impl Default for UpdatePatient {
    fn default() -> Self {
        Self {
            ahv_number: None,
            first_name: None,
            last_name: None,
            date_of_birth: None,
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
        }
    }
}

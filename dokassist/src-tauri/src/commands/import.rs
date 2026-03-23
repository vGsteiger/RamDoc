use crate::ahv::validate_ahv;
use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::import::{
    ColumnMapping, CsvPatientRow, CsvPreview, CsvWarning, ImportResult,
};
use crate::models::patient::CreatePatient;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Request to preview a CSV file before import
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCsvRequest {
    pub file_path: String,
}

/// Request to import CSV data with column mappings
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCsvRequest {
    pub file_path: String,
    pub column_mappings: Vec<ColumnMapping>,
}

/// Parse CSV file and return preview with detected column mappings
#[tauri::command]
pub async fn parse_csv_preview(
    state: tauri::State<'_, AppState>,
    request: PreviewCsvRequest,
) -> Result<CsvPreview, String> {
    // Validate file exists and is readable
    let path = Path::new(&request.file_path);
    if !path.exists() {
        return Err("File not found".to_string());
    }

    // Parse CSV file
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to read CSV file: {}", e))?;

    // Get headers
    let headers = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();

    if headers.is_empty() {
        return Err("CSV file has no headers".to_string());
    }

    // Read sample rows (first 5)
    let mut sample_rows = Vec::new();
    let mut total_rows = 0;
    let mut warnings = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        total_rows += 1;

        if idx < 5 {
            match result {
                Ok(record) => {
                    let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                    sample_rows.push(row);
                }
                Err(e) => {
                    warnings.push(CsvWarning {
                        row: idx + 2, // +1 for header, +1 for 1-based indexing
                        column: None,
                        message: format!("Failed to parse row: {}", e),
                    });
                }
            }
        }
    }

    // Detect column mappings based on header names
    let detected_mappings = detect_column_mappings(&headers);

    // Validate detected mappings
    let has_required_fields =
        detected_mappings.iter().any(|m| m.patient_field == "ahv_number")
            && detected_mappings.iter().any(|m| m.patient_field == "first_name")
            && detected_mappings.iter().any(|m| m.patient_field == "last_name")
            && detected_mappings.iter().any(|m| m.patient_field == "date_of_birth");

    if !has_required_fields {
        warnings.push(CsvWarning {
            row: 0,
            column: None,
            message: "Could not detect all required fields (AHV, first name, last name, date of birth). Please manually map columns.".to_string(),
        });
    }

    // Audit the preview action
    if let Ok(conn) = state.db.conn() {
        let _ = audit::log(
            &conn,
            AuditAction::View,
            "import",
            None,
            Some(&format!("preview: {} rows", total_rows)),
        );
    }

    Ok(CsvPreview {
        headers,
        sample_rows,
        total_rows,
        detected_mappings,
        warnings,
    })
}

/// Import CSV data using provided column mappings
#[tauri::command]
pub async fn import_csv_data(
    state: tauri::State<'_, AppState>,
    request: ImportCsvRequest,
) -> Result<ImportResult, String> {
    // Validate file exists
    let path = Path::new(&request.file_path);
    if !path.exists() {
        return Err("File not found".to_string());
    }

    // Validate column mappings
    let mapping_map: HashMap<String, String> = request
        .column_mappings
        .iter()
        .map(|m| (m.csv_header.clone(), m.patient_field.clone()))
        .collect();

    // Check required fields are mapped
    let required_fields = vec!["ahv_number", "first_name", "last_name", "date_of_birth"];
    for field in required_fields {
        if !mapping_map.values().any(|v| v == field) {
            return Err(format!("Required field '{}' is not mapped", field));
        }
    }

    // Parse CSV and import patients
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let headers = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();

    let mut imported_count = 0;
    let mut failed_count = 0;
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Get database connection
    let conn = state.db.conn().map_err(|e| e.to_string())?;

    for (idx, result) in reader.records().enumerate() {
        let row_num = idx + 2; // +1 for header, +1 for 1-based indexing

        match result {
            Ok(record) => {
                // Map CSV row to patient fields
                let patient_row = map_csv_row_to_patient(&headers, &record, &mapping_map);

                // Validate required fields
                if let Err(e) = validate_patient_row(&patient_row, row_num) {
                    errors.push(e);
                    failed_count += 1;
                    continue;
                }

                // Create patient
                match create_patient_from_row(&conn, patient_row, row_num) {
                    Ok(patient_id) => {
                        imported_count += 1;

                        // Log audit entry
                        let _ = audit::log(
                            &conn,
                            AuditAction::Import,
                            "patient",
                            Some(&patient_id),
                            Some(&format!("csv_row: {}", row_num)),
                        );
                    }
                    Err(warning) => {
                        errors.push(warning);
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                errors.push(CsvWarning {
                    row: row_num,
                    column: None,
                    message: format!("Failed to parse row: {}", e),
                });
                failed_count += 1;
            }
        }
    }

    // Log overall import result
    let _ = audit::log(
        &conn,
        AuditAction::Import,
        "import",
        None,
        Some(&format!(
            "imported: {}, failed: {}",
            imported_count, failed_count
        )),
    );

    Ok(ImportResult {
        success: failed_count == 0,
        imported_count,
        failed_count,
        warnings,
        errors,
    })
}

/// Detect column mappings from CSV headers
fn detect_column_mappings(headers: &[String]) -> Vec<ColumnMapping> {
    let mut mappings = Vec::new();

    for header in headers {
        let header_lower = header.to_lowercase();

        // Match common column names to patient fields
        let patient_field = if header_lower.contains("ahv") || header_lower.contains("avs") {
            Some("ahv_number")
        } else if header_lower.contains("first") && header_lower.contains("name")
            || header_lower.contains("vorname")
            || header_lower.contains("prenom")
        {
            Some("first_name")
        } else if header_lower.contains("last") && header_lower.contains("name")
            || header_lower.contains("nachname")
            || header_lower.contains("nom")
        {
            Some("last_name")
        } else if header_lower.contains("birth") || header_lower.contains("geburtsdatum") {
            Some("date_of_birth")
        } else if header_lower.contains("gender")
            || header_lower.contains("geschlecht")
            || header_lower.contains("sexe")
        {
            Some("gender")
        } else if header_lower.contains("address")
            || header_lower.contains("adresse")
            || header_lower.contains("strasse")
            || header_lower.contains("street")
        {
            Some("address")
        } else if header_lower.contains("phone")
            || header_lower.contains("tel")
            || header_lower.contains("telefon")
        {
            Some("phone")
        } else if header_lower.contains("email") || header_lower.contains("e-mail") {
            Some("email")
        } else if header_lower.contains("insurance")
            || header_lower.contains("versicherung")
            || header_lower.contains("assurance")
        {
            Some("insurance")
        } else if header_lower.contains("gp") && header_lower.contains("name")
            || header_lower.contains("hausarzt")
        {
            Some("gp_name")
        } else if header_lower.contains("gp") && header_lower.contains("address") {
            Some("gp_address")
        } else if header_lower.contains("note") || header_lower.contains("bemerkung") {
            Some("notes")
        } else {
            None
        };

        if let Some(field) = patient_field {
            mappings.push(ColumnMapping {
                csv_header: header.clone(),
                patient_field: field.to_string(),
            });
        }
    }

    mappings
}

/// Map a CSV row to patient fields using column mappings
fn map_csv_row_to_patient(
    headers: &[String],
    record: &csv::StringRecord,
    mapping_map: &HashMap<String, String>,
) -> CsvPatientRow {
    let mut patient_row = CsvPatientRow::default();

    for (idx, header) in headers.iter().enumerate() {
        if let Some(patient_field) = mapping_map.get(header) {
            if let Some(value) = record.get(idx) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    match patient_field.as_str() {
                        "ahv_number" => patient_row.ahv_number = Some(trimmed.to_string()),
                        "first_name" => patient_row.first_name = Some(trimmed.to_string()),
                        "last_name" => patient_row.last_name = Some(trimmed.to_string()),
                        "date_of_birth" => patient_row.date_of_birth = Some(trimmed.to_string()),
                        "gender" => patient_row.gender = Some(trimmed.to_string()),
                        "address" => patient_row.address = Some(trimmed.to_string()),
                        "phone" => patient_row.phone = Some(trimmed.to_string()),
                        "email" => patient_row.email = Some(trimmed.to_string()),
                        "insurance" => patient_row.insurance = Some(trimmed.to_string()),
                        "gp_name" => patient_row.gp_name = Some(trimmed.to_string()),
                        "gp_address" => patient_row.gp_address = Some(trimmed.to_string()),
                        "notes" => patient_row.notes = Some(trimmed.to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    patient_row
}

/// Validate a patient row has required fields
fn validate_patient_row(row: &CsvPatientRow, row_num: usize) -> Result<(), CsvWarning> {
    if row.ahv_number.is_none() {
        return Err(CsvWarning {
            row: row_num,
            column: Some("ahv_number".to_string()),
            message: "Missing required field: AHV number".to_string(),
        });
    }

    if row.first_name.is_none() {
        return Err(CsvWarning {
            row: row_num,
            column: Some("first_name".to_string()),
            message: "Missing required field: first name".to_string(),
        });
    }

    if row.last_name.is_none() {
        return Err(CsvWarning {
            row: row_num,
            column: Some("last_name".to_string()),
            message: "Missing required field: last name".to_string(),
        });
    }

    if row.date_of_birth.is_none() {
        return Err(CsvWarning {
            row: row_num,
            column: Some("date_of_birth".to_string()),
            message: "Missing required field: date of birth".to_string(),
        });
    }

    Ok(())
}

/// Create a patient from a CSV row
fn create_patient_from_row(
    conn: &rusqlite::Connection,
    row: CsvPatientRow,
    row_num: usize,
) -> Result<String, CsvWarning> {
    // Validate AHV number
    let ahv = row.ahv_number.unwrap(); // Safe because validated
    let normalized_ahv = validate_ahv(&ahv).map_err(|e| CsvWarning {
        row: row_num,
        column: Some("ahv_number".to_string()),
        message: format!("Invalid AHV number: {}", e),
    })?;

    // Create patient
    let input = CreatePatient {
        ahv_number: normalized_ahv,
        first_name: row.first_name.unwrap(), // Safe because validated
        last_name: row.last_name.unwrap(),    // Safe because validated
        date_of_birth: row.date_of_birth.unwrap(), // Safe because validated
        gender: row.gender,
        address: row.address,
        phone: row.phone,
        email: row.email,
        insurance: row.insurance,
        gp_name: row.gp_name,
        gp_address: row.gp_address,
        notes: row.notes,
    };

    let patient = crate::models::patient::create_patient(conn, input).map_err(|e| CsvWarning {
        row: row_num,
        column: None,
        message: format!("Failed to create patient: {}", e),
    })?;

    Ok(patient.id)
}

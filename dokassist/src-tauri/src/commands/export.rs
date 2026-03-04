use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::filesystem;
use crate::models::patient::Patient;
use crate::models::{diagnosis, file_record, medication, patient, report, session};
use crate::state::{AppState, AuthState};
use rusqlite::Connection;
use serde::Serialize;
use std::io::Write;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Serialize)]
struct PatientExport {
    patient: Patient,
    sessions: Vec<serde_json::Value>,
    diagnoses: Vec<serde_json::Value>,
    medications: Vec<serde_json::Value>,
    reports: Vec<serde_json::Value>,
    files: Vec<serde_json::Value>,
}

/// Export all patient data to a ZIP file containing:
/// - One directory per patient (patient_LastName_FirstName_UUID)
/// - patient.json with all patient metadata
/// - Decrypted files in their original format
#[tauri::command]
pub async fn export_all_patient_data(state: State<'_, AppState>) -> Result<Vec<u8>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let base_dir = state.data_dir.clone();

    let fs_key: [u8; 32] = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::Validation("Auth state mutex poisoned".to_string()))?;
        match &*auth {
            AuthState::Unlocked { fs_key, .. } => **fs_key,
            _ => return Err(AppError::AuthRequired),
        }
    };

    // Create a ZIP file in memory
    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6));

        // Get all patients
        let patients = patient::list_patients(&conn, u32::MAX, 0)?;

        for patient in patients {
            let patient_dir = format!(
                "patient_{}_{}_{}",
                patient.last_name.replace(['/', '\\', ':'], "_"),
                patient.first_name.replace(['/', '\\', ':'], "_"),
                patient.id
            );

            // Collect all data for this patient
            let export_data = collect_patient_data(&conn, &patient.id)?;

            // Write patient metadata JSON
            let json_path = format!("{}/patient.json", patient_dir);
            zip.start_file(&json_path, options)
                .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
            let json_data = serde_json::to_string_pretty(&export_data)
                .map_err(|e| AppError::Validation(format!("JSON serialization failed: {}", e)))?;
            zip.write_all(json_data.as_bytes())?;

            // Export all files for this patient
            let files = file_record::list_files_for_patient(&conn, &patient.id)?;
            for file in files {
                // Read and decrypt the file
                match filesystem::read_file(&base_dir, &fs_key, &file.vault_path) {
                    Ok(plaintext) => {
                        let file_path = format!("{}/files/{}", patient_dir, file.filename);
                        zip.start_file(&file_path, options)
                            .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
                        zip.write_all(&plaintext)?;
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to read file {} for patient {}: {}",
                            file.vault_path,
                            patient.id,
                            e
                        );
                        // Continue with other files even if one fails
                    }
                }
            }
        }

        zip.finish()
            .map_err(|e| AppError::Validation(format!("ZIP finalize error: {}", e)))?;
    }

    // Log the export action
    audit::log(
        &conn,
        AuditAction::Export,
        "all_patients",
        None,
        Some("Emergency export to ZIP"),
    )?;

    Ok(zip_buffer)
}

fn collect_patient_data(conn: &Connection, patient_id: &str) -> Result<PatientExport, AppError> {
    let patient = patient::get_patient(conn, patient_id)?;

    // Get all related data
    let sessions = session::list_sessions_for_patient(conn, patient_id, u32::MAX, 0)?
        .into_iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    let diagnoses = diagnosis::list_diagnoses_for_patient(conn, patient_id, u32::MAX, 0)?
        .into_iter()
        .map(|d| serde_json::to_value(d).unwrap())
        .collect();

    let medications = medication::list_medications_for_patient(conn, patient_id, u32::MAX, 0)?
        .into_iter()
        .map(|m| serde_json::to_value(m).unwrap())
        .collect();

    let reports = report::list_reports_for_patient(conn, patient_id, u32::MAX, 0)?
        .into_iter()
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();

    let files = file_record::list_files_for_patient(conn, patient_id)?
        .into_iter()
        .map(|f| serde_json::to_value(f).unwrap())
        .collect();

    Ok(PatientExport {
        patient,
        sessions,
        diagnoses,
        medications,
        reports,
        files,
    })
}

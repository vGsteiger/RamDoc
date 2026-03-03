use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::filesystem;
use crate::models::{diagnosis, file_record, medication, patient, report, session};
use crate::state::{AppState, AuthState};
use rusqlite::Connection;
use serde::Serialize;
use std::io::Write;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Aggregated patient data for export
#[derive(Debug, Serialize)]
struct PatientExportData {
    patient: patient::Patient,
    sessions: Vec<session::Session>,
    diagnoses: Vec<diagnosis::Diagnosis>,
    medications: Vec<medication::Medication>,
    reports: Vec<report::Report>,
    files: Vec<FileWithData>,
}

#[derive(Debug, Serialize)]
struct FileWithData {
    metadata: file_record::FileRecord,
    #[serde(skip_serializing)]
    data: Vec<u8>,
}

/// Export all patient data to a ZIP archive
/// Returns the ZIP file as a byte array
#[tauri::command]
pub async fn export_all_patient_data(state: State<'_, AppState>) -> Result<Vec<u8>, AppError> {
    // Verify auth state and get keys
    let auth = state.auth.lock().unwrap();
    let (db_key, fs_key) = match *auth {
        AuthState::Unlocked { db_key, fs_key } => (db_key, fs_key),
        _ => return Err(AppError::AuthRequired),
    };
    drop(auth); // Release lock early

    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Get all patients
    let patients = patient::list_patients(&conn, 10000, 0)?;

    // Create ZIP archive in memory
    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Create README
        let readme = format!(
            "DokAssist Emergency Export\n\
             ===========================\n\n\
             Export Date: {}\n\
             Total Patients: {}\n\n\
             This export contains all patient data in structured JSON format.\n\
             Each patient has their own directory with:\n\
             - patient.json: Patient demographics and metadata\n\
             - sessions.json: All clinical sessions\n\
             - diagnoses.json: All diagnoses\n\
             - medications.json: All medications\n\
             - reports.json: All generated reports\n\
             - files/: All uploaded files (decrypted)\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            patients.len()
        );
        zip.start_file("README.txt", options)?;
        zip.write_all(readme.as_bytes())?;

        // Export each patient
        for patient in &patients {
            export_patient_to_zip(&mut zip, &conn, &state.data_dir, &fs_key, patient, options)?;
        }

        zip.finish()?;
    }

    // Audit log
    audit::log(
        &conn,
        AuditAction::View,
        "export",
        None,
        Some(&format!("Emergency export: {} patients", patients.len())),
    )?;

    Ok(zip_buffer)
}

/// Export a single patient's data to the ZIP archive
fn export_patient_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    conn: &Connection,
    data_dir: &std::path::Path,
    fs_key: &[u8; 32],
    patient: &patient::Patient,
    options: SimpleFileOptions,
) -> Result<(), AppError> {
    let patient_dir = format!("patient_{}_{}", patient.last_name, patient.first_name);
    let patient_id = &patient.id;

    // Gather all related data (using large limits to get all records)
    let sessions = session::list_sessions_for_patient(conn, patient_id, 10000, 0)?;
    let diagnoses = diagnosis::list_diagnoses_for_patient(conn, patient_id, 10000, 0)?;
    let medications = medication::list_medications_for_patient(conn, patient_id, 10000, 0)?;
    let reports = report::list_reports_for_patient(conn, patient_id, 10000, 0)?;
    let file_records = file_record::list_files_for_patient(conn, patient_id)?;

    // Export patient.json
    zip.start_file(format!("{}/patient.json", patient_dir), options)?;
    let patient_json = serde_json::to_string_pretty(patient)?;
    zip.write_all(patient_json.as_bytes())?;

    // Export sessions.json
    if !sessions.is_empty() {
        zip.start_file(format!("{}/sessions.json", patient_dir), options)?;
        let sessions_json = serde_json::to_string_pretty(&sessions)?;
        zip.write_all(sessions_json.as_bytes())?;
    }

    // Export diagnoses.json
    if !diagnoses.is_empty() {
        zip.start_file(format!("{}/diagnoses.json", patient_dir), options)?;
        let diagnoses_json = serde_json::to_string_pretty(&diagnoses)?;
        zip.write_all(diagnoses_json.as_bytes())?;
    }

    // Export medications.json
    if !medications.is_empty() {
        zip.start_file(format!("{}/medications.json", patient_dir), options)?;
        let medications_json = serde_json::to_string_pretty(&medications)?;
        zip.write_all(medications_json.as_bytes())?;
    }

    // Export reports.json
    if !reports.is_empty() {
        zip.start_file(format!("{}/reports.json", patient_dir), options)?;
        let reports_json = serde_json::to_string_pretty(&reports)?;
        zip.write_all(reports_json.as_bytes())?;
    }

    // Export files
    for file_record in file_records {
        // Decrypt and add file to ZIP
        let file_data = filesystem::read_file(data_dir, fs_key, &file_record.vault_path)?;

        let file_path = format!("{}/files/{}", patient_dir, file_record.filename);
        zip.start_file(file_path, options)?;
        zip.write_all(&file_data)?;
    }

    Ok(())
}

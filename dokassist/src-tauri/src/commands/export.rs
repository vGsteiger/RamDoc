use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::filesystem;
use crate::models::patient::Patient;
use crate::models::{diagnosis, file_record, medication, patient, report, session};
use crate::state::{AppState, AuthState};
use chrono::NaiveDate;
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

/// Export a patient summary to PDF format
#[tauri::command]
pub async fn export_patient_pdf(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<Vec<u8>, AppError> {
    let pool = state.get_db()?;

    // Get all patient data under a short-lived DB connection
    let (patient, sessions, diagnoses, medications) = {
        let conn = pool.conn()?;
        let patient = patient::get_patient(&conn, &patient_id)?;
        let sessions = session::list_sessions_for_patient(&conn, &patient_id, u32::MAX, 0)?;
        let diagnoses = diagnosis::list_diagnoses_for_patient(&conn, &patient_id, u32::MAX, 0)?;
        let medications =
            medication::list_medications_for_patient(&conn, &patient_id, u32::MAX, 0)?;
        (patient, sessions, diagnoses, medications)
    };

    // Generate PDF in a blocking task to avoid blocking Tokio runtime
    let pdf_bytes = tokio::task::spawn_blocking(move || {
        generate_patient_summary_pdf(patient, sessions, diagnoses, medications)
    })
    .await
    .map_err(|e| AppError::Validation(format!("PDF generation task failed: {}", e)))??;

    // Audit log with a fresh connection
    {
        let conn = pool.conn()?;
        audit::log(
            &conn,
            AuditAction::Export,
            "patient",
            Some(&patient_id),
            Some("Exported patient summary to PDF"),
        )?;
    }

    Ok(pdf_bytes)
}

fn generate_patient_summary_pdf(
    patient: Patient,
    sessions: Vec<session::Session>,
    diagnoses: Vec<diagnosis::Diagnosis>,
    medications: Vec<medication::Medication>,
) -> Result<Vec<u8>, AppError> {
    use printpdf::{
        BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt,
        TextItem,
    };

    // Format dates
    let dob = NaiveDate::parse_from_str(&patient.date_of_birth, "%Y-%m-%d")
        .map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_else(|_| patient.date_of_birth.clone());

    let font = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let font_bold = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);

    // Helper: emit a single line of text at (x_mm, y_mm) with given font & size
    let text_op = |text: String, size: f32, x: Mm, y: Mm, fh: &PdfFontHandle| -> Vec<Op> {
        vec![
            Op::StartTextSection,
            Op::SetFont {
                font: fh.clone(),
                size: Pt(size),
            },
            Op::SetTextCursor {
                pos: Point::new(x, y),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text)],
            },
            Op::EndTextSection,
        ]
    };

    let mut doc = PdfDocument::new("Patient Summary");
    let page_w = Mm(210.0);
    let page_h = Mm(297.0);
    let left = Mm(20.0);
    let lh = Mm(5.0);

    let mut all_ops: Vec<Op> = Vec::new();
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut y = Mm(270.0);

    let flush_page = |ops: Vec<Op>, pages: &mut Vec<PdfPage>| {
        pages.push(PdfPage::new(page_w, page_h, ops));
    };

    // Title
    all_ops.extend(text_op(
        "Patient Summary".to_string(),
        24.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh * 2.0;

    // Patient Demographics Section
    all_ops.extend(text_op(
        "Patient Demographics".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    all_ops.extend(text_op(
        format!("Name: {} {}", patient.first_name, patient.last_name),
        11.0,
        left,
        y,
        &font,
    ));
    y -= lh;

    all_ops.extend(text_op(
        format!("Date of Birth: {}", dob),
        11.0,
        left,
        y,
        &font,
    ));
    y -= lh;

    all_ops.extend(text_op(
        format!("AHV Number: {}", patient.ahv_number),
        11.0,
        left,
        y,
        &font,
    ));
    y -= lh;

    if let Some(gender) = &patient.gender {
        all_ops.extend(text_op(format!("Gender: {}", gender), 11.0, left, y, &font));
        y -= lh;
    }

    if let Some(phone) = &patient.phone {
        all_ops.extend(text_op(format!("Phone: {}", phone), 11.0, left, y, &font));
        y -= lh;
    }

    if let Some(email) = &patient.email {
        all_ops.extend(text_op(format!("Email: {}", email), 11.0, left, y, &font));
        y -= lh;
    }

    if let Some(address) = &patient.address {
        all_ops.extend(text_op(
            format!("Address: {}", address),
            11.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    }

    if let Some(insurance) = &patient.insurance {
        all_ops.extend(text_op(
            format!("Insurance: {}", insurance),
            11.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    }

    if let Some(gp_name) = &patient.gp_name {
        all_ops.extend(text_op(
            format!("GP Name: {}", gp_name),
            11.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    }

    y -= lh;

    // Active Diagnoses Section
    if y.0 < 50.0 {
        flush_page(std::mem::take(&mut all_ops), &mut pages);
        y = Mm(270.0);
    }

    all_ops.extend(text_op(
        "Active Diagnoses".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    let active_diagnoses: Vec<_> = diagnoses
        .iter()
        .filter(|d| d.status.to_lowercase() == "active")
        .collect();

    if active_diagnoses.is_empty() {
        all_ops.extend(text_op(
            "No active diagnoses".to_string(),
            10.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    } else {
        for diagnosis in active_diagnoses {
            if y.0 < 30.0 {
                flush_page(std::mem::take(&mut all_ops), &mut pages);
                y = Mm(270.0);
            }

            let diagnosed_date = NaiveDate::parse_from_str(&diagnosis.diagnosed_date, "%Y-%m-%d")
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_else(|_| diagnosis.diagnosed_date.clone());

            all_ops.extend(text_op(
                format!(
                    "{} - {} ({})",
                    diagnosis.icd10_code, diagnosis.description, diagnosed_date
                ),
                10.0,
                left,
                y,
                &font,
            ));
            y -= lh;
        }
    }

    y -= lh;

    // Current Medications Section
    if y.0 < 50.0 {
        flush_page(std::mem::take(&mut all_ops), &mut pages);
        y = Mm(270.0);
    }

    all_ops.extend(text_op(
        "Current Medications".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    let current_medications: Vec<_> = medications
        .iter()
        .filter(|m| m.end_date.is_none())
        .collect();

    if current_medications.is_empty() {
        all_ops.extend(text_op(
            "No current medications".to_string(),
            10.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    } else {
        for medication in current_medications {
            if y.0 < 30.0 {
                flush_page(std::mem::take(&mut all_ops), &mut pages);
                y = Mm(270.0);
            }

            all_ops.extend(text_op(
                format!(
                    "{} - {} {}",
                    medication.substance, medication.dosage, medication.frequency
                ),
                10.0,
                left,
                y,
                &font,
            ));
            y -= lh;
        }
    }

    y -= lh;

    // Sessions Section (chronological)
    if y.0 < 50.0 {
        flush_page(std::mem::take(&mut all_ops), &mut pages);
        y = Mm(270.0);
    }

    all_ops.extend(text_op(
        "Session History".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    if sessions.is_empty() {
        all_ops.extend(text_op(
            "No sessions recorded".to_string(),
            10.0,
            left,
            y,
            &font,
        ));
        y -= lh;
    } else {
        for session in &sessions {
            if y.0 < 30.0 {
                flush_page(std::mem::take(&mut all_ops), &mut pages);
                y = Mm(270.0);
            }

            let session_date = NaiveDate::parse_from_str(&session.session_date, "%Y-%m-%d")
                .map(|d| d.format("%d.%m.%Y").to_string())
                .unwrap_or_else(|_| session.session_date.clone());

            let duration = session
                .duration_minutes
                .map(|d| format!(" ({}min)", d))
                .unwrap_or_default();

            all_ops.extend(text_op(
                format!("{} - {}{}", session_date, session.session_type, duration),
                10.0,
                left,
                y,
                &font,
            ));
            y -= lh;
        }
    }

    // Flush last page
    flush_page(all_ops, &mut pages);
    doc.pages = pages;

    let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
    Ok(pdf_bytes)
}

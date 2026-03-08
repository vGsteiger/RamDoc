use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::patient::{self, Patient};
use crate::models::report::{self, CreateReport, Report, UpdateReport};
use crate::state::AppState;
use chrono::{NaiveDate, NaiveDateTime};
use docx_rs::*;
use printpdf::*;
use tauri::State;

#[tauri::command]
pub async fn create_report(
    state: State<'_, AppState>,
    input: CreateReport,
) -> Result<Report, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let report = report::create_report(&tx, input)?;

    // PKG-6: Audit logging
    audit::log(&tx, AuditAction::Create, "report", Some(&report.id), None)?;

    tx.commit()?;

    Ok(report)
}

#[tauri::command]
pub async fn get_report(state: State<'_, AppState>, id: String) -> Result<Report, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let report = report::get_report(&conn, &id)?;

    // PKG-6: Audit logging
    audit::log(&conn, AuditAction::View, "report", Some(&id), None)?;

    Ok(report)
}

#[tauri::command]
pub async fn list_reports(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Report>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let reports = report::list_reports_for_patient(&conn, &patient_id, limit, offset)?;

    Ok(reports)
}

#[tauri::command]
pub async fn update_report(
    state: State<'_, AppState>,
    id: String,
    input: UpdateReport,
) -> Result<Report, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let report = report::update_report(&tx, &id, input)?;

    // PKG-6: Audit logging
    audit::log(&tx, AuditAction::Update, "report", Some(&id), None)?;

    tx.commit()?;

    Ok(report)
}

#[tauri::command]
pub async fn delete_report(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    report::delete_report(&tx, &id)?;

    // PKG-6: Audit logging
    audit::log(&tx, AuditAction::Delete, "report", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}

/// Export a report to PDF format
#[tauri::command]
pub async fn export_report_to_pdf(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<Vec<u8>, AppError> {
    let pool = state.get_db()?;

    // Get report and patient data under a short-lived DB connection
    let (report, patient) = {
        let conn = pool.conn()?;
        let report = report::get_report(&conn, &report_id)?;
        let patient = patient::get_patient(&conn, &report.patient_id)?;
        (report, patient)
    };

    // Generate PDF in a blocking task to avoid blocking Tokio runtime
    let pdf_bytes = tokio::task::spawn_blocking(move || generate_pdf_bytes(report, patient))
        .await
        .map_err(|e| AppError::Validation(format!("PDF generation task failed: {}", e)))??;

    // Audit log with a fresh connection
    {
        let conn = pool.conn()?;
        audit::log(
            &conn,
            AuditAction::Export,
            "report",
            Some(&report_id),
            Some("Exported to PDF"),
        )?;
    }

    Ok(pdf_bytes)
}

fn generate_pdf_bytes(report: Report, patient: Patient) -> Result<Vec<u8>, AppError> {
    use printpdf::{Op, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem};

    // Format dates
    let generated_at = NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%dT%H:%M:%S%.f"))
        .map(|dt| dt.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_else(|_| report.generated_at.clone());

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

    let mut doc = PdfDocument::new("Report");
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
        format_report_type(&report.report_type),
        24.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh * 2.0;

    // Patient information header
    all_ops.extend(text_op(
        "Patient Information".to_string(),
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
    y -= lh * 2.0;

    // Report metadata
    all_ops.extend(text_op(
        format!("Generated: {}", generated_at),
        10.0,
        left,
        y,
        &font,
    ));
    y -= lh * 2.0;

    // Report content header
    all_ops.extend(text_op(
        "Report Content".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    // Content lines
    let max_chars = 90usize;
    for paragraph in report.content.split('\n') {
        if paragraph.is_empty() {
            y -= lh * 0.5;
            continue;
        }

        let char_indices: Vec<(usize, char)> = paragraph.char_indices().collect();
        let mut start_idx = 0;

        while start_idx < char_indices.len() {
            let end_idx = (start_idx + max_chars).min(char_indices.len());
            let break_idx = if end_idx < char_indices.len() {
                char_indices[start_idx..end_idx]
                    .iter()
                    .rposition(|(_, c)| *c == ' ')
                    .map(|pos| start_idx + pos)
                    .unwrap_or(end_idx)
            } else {
                end_idx
            };

            let byte_start = if start_idx == 0 {
                0
            } else {
                char_indices[start_idx].0
            };
            let byte_end = if break_idx < char_indices.len() {
                char_indices[break_idx].0
            } else {
                paragraph.len()
            };
            let chunk = paragraph[byte_start..byte_end].trim().to_string();

            // New page if needed
            if y.0 < 30.0 {
                flush_page(std::mem::take(&mut all_ops), &mut pages);
                y = Mm(270.0);
            }

            all_ops.extend(text_op(chunk, 10.0, left, y, &font));
            y -= lh;

            start_idx = break_idx;
            while start_idx < char_indices.len() && char_indices[start_idx].1.is_whitespace() {
                start_idx += 1;
            }
        }
    }

    // Flush last page
    flush_page(all_ops, &mut pages);
    doc.pages = pages;

    let pdf_bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
    Ok(pdf_bytes)
}

/// Export a report to DOCX format
#[tauri::command]
pub async fn export_report_to_docx(
    state: State<'_, AppState>,
    report_id: String,
) -> Result<Vec<u8>, AppError> {
    let pool = state.get_db()?;

    // Get report and patient data under a short-lived DB connection
    let (report, patient) = {
        let conn = pool.conn()?;
        let report = report::get_report(&conn, &report_id)?;
        let patient = patient::get_patient(&conn, &report.patient_id)?;
        (report, patient)
    };

    // Generate DOCX in a blocking task to avoid blocking Tokio runtime
    let docx_bytes = tokio::task::spawn_blocking(move || generate_docx_bytes(report, patient))
        .await
        .map_err(|e| AppError::Validation(format!("DOCX generation task failed: {}", e)))??;

    // Audit log with a fresh connection
    {
        let conn = pool.conn()?;
        audit::log(
            &conn,
            AuditAction::Export,
            "report",
            Some(&report_id),
            Some("Exported to DOCX"),
        )?;
    }

    Ok(docx_bytes)
}

fn generate_docx_bytes(report: Report, patient: Patient) -> Result<Vec<u8>, AppError> {
    // Format dates
    let generated_at = NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%dT%H:%M:%S%.f"))
        .map(|dt| dt.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_else(|_| report.generated_at.clone());

    let dob = NaiveDate::parse_from_str(&patient.date_of_birth, "%Y-%m-%d")
        .map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_else(|_| patient.date_of_birth.clone());

    // Create DOCX document
    let mut docx = Docx::new();

    // Title
    docx = docx.add_paragraph(
        Paragraph::new().add_run(
            Run::new()
                .add_text(format_report_type(&report.report_type))
                .bold()
                .size(48),
        ),
    );

    // Empty line
    docx = docx.add_paragraph(Paragraph::new());

    // Patient information section
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text("Patient Information").bold().size(28)),
    );

    docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!(
        "Name: {} {}",
        patient.first_name, patient.last_name
    ))));

    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(format!("Date of Birth: {}", dob))),
    );

    docx = docx.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(format!("AHV Number: {}", patient.ahv_number))),
    );

    // Empty line
    docx = docx.add_paragraph(Paragraph::new());

    // Report metadata
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(format!("Generated: {}", generated_at))),
    );

    // Empty line
    docx = docx.add_paragraph(Paragraph::new());

    // Report content section
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text("Report Content").bold().size(28)),
    );

    // Add report content, preserving paragraphs
    for paragraph_text in report.content.split('\n') {
        if paragraph_text.trim().is_empty() {
            docx = docx.add_paragraph(Paragraph::new());
        } else {
            docx =
                docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph_text)));
        }
    }

    // Convert DOCX to bytes (pack requires Write + Seek, so use Cursor)
    let mut cursor = std::io::Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .map_err(|e| AppError::Validation(format!("Failed to generate DOCX: {}", e)))?;

    Ok(cursor.into_inner())
}

fn format_report_type(report_type: &str) -> String {
    match report_type {
        "Befundbericht" => "Befundbericht".to_string(),
        "Verlaufsbericht" => "Verlaufsbericht".to_string(),
        "Ueberweisungsschreiben" => "Überweisungsschreiben".to_string(),
        _ => report_type.to_string(),
    }
}

use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::patient::{self, Patient};
use crate::models::report::{self, CreateReport, Report, UpdateReport};
use crate::state::AppState;
use chrono::{NaiveDate, NaiveDateTime};
use docx_rs::*;
use printpdf::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
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

/// Load an external TTF font from the macOS system font directory.
/// Falls back to None if the font file is not present.
fn load_system_font(filename: &str) -> Option<ParsedFont> {
    let path = format!("/System/Library/Fonts/Supplemental/{}", filename);
    let bytes = std::fs::read(&path).ok()?;
    ParsedFont::from_bytes(&bytes, 0, &mut Vec::new())
}

/// A line to render in the PDF, with style information derived from markdown.
enum PdfLine {
    Heading { text: String, level: u8 },
    Body(String),
    Separator,
    Blank,
}

/// Convert markdown content into a flat list of renderable lines.
fn markdown_to_pdf_lines(markdown: &str) -> Vec<PdfLine> {
    let mut lines = Vec::new();
    let parser = Parser::new_ext(markdown, Options::empty());

    let mut current_text = String::new();
    let mut heading_level: Option<u8> = None;
    let mut in_strong = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    _ => 3,
                });
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                let text = current_text.trim().to_string();
                if !text.is_empty() {
                    lines.push(PdfLine::Heading {
                        text,
                        level: heading_level.unwrap_or(3),
                    });
                }
                heading_level = None;
                current_text.clear();
            }
            Event::Start(Tag::Paragraph) => {
                current_text.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                let text = current_text.trim().to_string();
                if !text.is_empty() {
                    lines.push(PdfLine::Body(text));
                }
                lines.push(PdfLine::Blank);
                current_text.clear();
            }
            Event::Start(Tag::Strong) | Event::Start(Tag::Emphasis) => {
                in_strong = true;
            }
            Event::End(TagEnd::Strong) | Event::End(TagEnd::Emphasis) => {
                in_strong = false;
            }
            Event::Text(text) => {
                current_text.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                current_text.push(' ');
            }
            Event::Rule => {
                lines.push(PdfLine::Separator);
            }
            _ => {}
        }
        let _ = in_strong; // used implicitly via current_text accumulation
    }

    lines
}

fn generate_pdf_bytes(report: Report, patient: Patient) -> Result<Vec<u8>, AppError> {
    use printpdf::{Op, PdfPage, PdfSaveOptions, Point, Pt, TextItem};

    // Format dates
    let generated_at = NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%dT%H:%M:%S%.f"))
        .map(|dt| dt.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_else(|_| report.generated_at.clone());

    let dob = NaiveDate::parse_from_str(&patient.date_of_birth, "%Y-%m-%d")
        .map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_else(|_| patient.date_of_birth.clone());

    let mut doc = PdfDocument::new("Report");

    // Load Unicode-capable fonts; fall back to builtins if not found
    let (font, font_bold) = if let (Some(regular), Some(bold)) = (
        load_system_font("Arial.ttf"),
        load_system_font("Arial Bold.ttf"),
    ) {
        let id_regular = doc.add_font(&regular);
        let id_bold = doc.add_font(&bold);
        (
            PdfFontHandle::External(id_regular),
            PdfFontHandle::External(id_bold),
        )
    } else {
        (
            PdfFontHandle::Builtin(BuiltinFont::Helvetica),
            PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
        )
    };

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

    let page_w = Mm(210.0);
    let page_h = Mm(297.0);
    let left = Mm(20.0);
    let lh = Mm(5.0);
    let max_chars = 90usize;

    let mut all_ops: Vec<Op> = Vec::new();
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut y = Mm(270.0);

    let flush_page = |ops: Vec<Op>, pages: &mut Vec<PdfPage>| {
        pages.push(PdfPage::new(page_w, page_h, ops));
    };

    // Helper: word-wrap a text string and emit lines, mutating y
    let emit_wrapped = |text: &str,
                        size: f32,
                        fh: &PdfFontHandle,
                        all_ops: &mut Vec<Op>,
                        pages: &mut Vec<PdfPage>,
                        y: &mut Mm| {
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();
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

            let byte_start = char_indices[start_idx].0;
            let byte_end = if break_idx < char_indices.len() {
                char_indices[break_idx].0
            } else {
                text.len()
            };
            let chunk = text[byte_start..byte_end].trim().to_string();

            if y.0 < 30.0 {
                flush_page(std::mem::take(all_ops), pages);
                *y = Mm(270.0);
            }

            all_ops.extend(text_op(chunk, size, left, *y, fh));
            *y -= lh;

            start_idx = break_idx;
            while start_idx < char_indices.len() && char_indices[start_idx].1.is_whitespace() {
                start_idx += 1;
            }
        }
    };

    // Title
    emit_wrapped(
        &format_report_type(&report.report_type),
        24.0,
        &font_bold,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    y -= lh;

    // Patient information header
    all_ops.extend(text_op(
        "Patienteninformation".to_string(),
        14.0,
        left,
        y,
        &font_bold,
    ));
    y -= lh;

    emit_wrapped(
        &format!("Name: {} {}", patient.first_name, patient.last_name),
        11.0,
        &font,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    emit_wrapped(
        &format!("Geburtsdatum: {}", dob),
        11.0,
        &font,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    emit_wrapped(
        &format!("AHV-Nummer: {}", patient.ahv_number),
        11.0,
        &font,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    y -= lh;

    emit_wrapped(
        &format!("Erstellt: {}", generated_at),
        10.0,
        &font,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    y -= lh;

    // Report content — parse markdown
    for line in markdown_to_pdf_lines(&report.content) {
        if y.0 < 30.0 {
            flush_page(std::mem::take(&mut all_ops), &mut pages);
            y = Mm(270.0);
        }
        match line {
            PdfLine::Heading { text, level } => {
                let size = if level == 1 {
                    16.0
                } else if level == 2 {
                    14.0
                } else {
                    12.0
                };
                emit_wrapped(&text, size, &font_bold, &mut all_ops, &mut pages, &mut y);
                y -= lh * 0.3;
            }
            PdfLine::Body(text) => {
                emit_wrapped(&text, 10.0, &font, &mut all_ops, &mut pages, &mut y);
            }
            PdfLine::Separator | PdfLine::Blank => {
                y -= lh * 0.5;
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

/// A segment of text within a paragraph, with optional bold/italic styling.
struct DocxSpan {
    text: String,
    bold: bool,
    italic: bool,
}

/// Convert markdown content into DOCX paragraphs, preserving inline bold/italic.
fn markdown_to_docx(markdown: &str, docx: Docx) -> Docx {
    let mut docx = docx;
    let parser = Parser::new_ext(markdown, Options::empty());

    // Accumulator for the current paragraph's spans
    let mut spans: Vec<DocxSpan> = Vec::new();
    let mut in_strong = false;
    let mut in_em = false;
    let mut heading_level: Option<u8> = None;
    let mut in_paragraph = false;

    let flush_paragraph = |spans: Vec<DocxSpan>, heading_level: Option<u8>, docx: &mut Docx| {
        if spans.is_empty() {
            return;
        }
        let mut para = Paragraph::new();
        let is_heading = heading_level.is_some();
        for span in spans {
            let mut run = Run::new().add_text(span.text);
            if span.bold || is_heading {
                run = run.bold();
            }
            if span.italic {
                run = run.italic();
            }
            let size = match heading_level {
                Some(1) => 40,
                Some(2) => 34,
                Some(3) => 28,
                _ => 22,
            };
            run = run.size(size);
            para = para.add_run(run);
        }
        *docx = std::mem::take(docx).add_paragraph(para);
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    _ => 3,
                });
                spans.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_paragraph(std::mem::take(&mut spans), heading_level, &mut docx);
                heading_level = None;
            }
            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
                spans.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                flush_paragraph(std::mem::take(&mut spans), None, &mut docx);
                docx = docx.add_paragraph(Paragraph::new());
                in_paragraph = false;
            }
            Event::Start(Tag::Strong) => {
                in_strong = true;
            }
            Event::End(TagEnd::Strong) => {
                in_strong = false;
            }
            Event::Start(Tag::Emphasis) => {
                in_em = true;
            }
            Event::End(TagEnd::Emphasis) => {
                in_em = false;
            }
            Event::Text(text) => {
                spans.push(DocxSpan {
                    text: text.to_string(),
                    bold: in_strong,
                    italic: in_em,
                });
            }
            Event::SoftBreak | Event::HardBreak => {
                spans.push(DocxSpan {
                    text: " ".to_string(),
                    bold: in_strong,
                    italic: in_em,
                });
            }
            Event::Rule => {
                // Horizontal rule → blank paragraph
                docx = docx.add_paragraph(Paragraph::new());
            }
            _ => {}
        }
        let _ = in_paragraph; // suppress unused warning
    }

    // Flush any remaining spans (e.g. content not wrapped in a paragraph tag)
    flush_paragraph(std::mem::take(&mut spans), None, &mut docx);

    docx
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
    docx = docx.add_paragraph(Paragraph::new());

    // Patient information
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text("Patienteninformation").bold().size(28)),
    );
    docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!(
        "Name: {} {}",
        patient.first_name, patient.last_name
    ))));
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(format!("Geburtsdatum: {}", dob))),
    );
    docx = docx.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(format!("AHV-Nummer: {}", patient.ahv_number))),
    );
    docx = docx.add_paragraph(Paragraph::new());
    docx = docx.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(format!("Erstellt: {}", generated_at))),
    );
    docx = docx.add_paragraph(Paragraph::new());

    // Report content — parse markdown for proper formatting
    docx = markdown_to_docx(&report.content, docx);

    // Convert DOCX to bytes
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

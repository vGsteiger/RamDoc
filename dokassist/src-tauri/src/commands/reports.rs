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
#[derive(Debug, PartialEq)]
enum PdfLine {
    Heading { text: String, level: u8 },
    Body(String),
    Separator,
    Blank,
}

const CLINICAL_SECTION_HEADINGS: &[&str] = &[
    "anamnese",
    "beurteilung",
    "diagnosen",
    "diagnose",
    "fragestellung",
    "medikation",
    "aktuelle medikation",
    "psychischer befund",
    "aktueller befund",
    "relevante anamnese",
    "bisheriger verlauf",
    "bisherige behandlung",
    "bisherige behandlungen",
    "therapie",
    "therapieverlauf",
    "überweisungsgrund",
    "überweisungsgrund und fragestellung",
    "zuweisungsgrund",
    "zuweisungsgrund und fragestellung",
    "erbetenes vorgehen",
    "weiteres vorgehen",
    "vorgehen",
];

fn is_plain_heading(text: &str) -> bool {
    let trimmed = text.trim().trim_end_matches(':');
    let normalized = trimmed.to_lowercase();
    let word_count = trimmed.split_whitespace().count();

    (word_count <= 8
        && trimmed.chars().count() <= 80
        && CLINICAL_SECTION_HEADINGS
            .iter()
            .any(|heading| normalized == *heading))
        || (normalized.starts_with("betreff:") && trimmed.chars().count() <= 120)
}

/// Convert generated plain text or clinician-authored markdown into renderable blocks.
///
/// The model is intentionally asked for plain text, so standalone clinical section names
/// must be recognized even when they do not carry Markdown `#` markers.
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

    for line in &mut lines {
        if let PdfLine::Body(text) = line {
            if is_plain_heading(text) {
                *line = PdfLine::Heading {
                    text: text.clone(),
                    level: 2,
                };
            }
        }
    }

    if lines
        .iter()
        .any(|line| matches!(line, PdfLine::Heading { .. }))
    {
        return lines;
    }

    let mut plain_lines = Vec::new();
    let mut paragraph = Vec::new();
    let flush_paragraph = |paragraph: &mut Vec<&str>, output: &mut Vec<PdfLine>| {
        if !paragraph.is_empty() {
            output.push(PdfLine::Body(paragraph.join(" ")));
            output.push(PdfLine::Blank);
            paragraph.clear();
        }
    };

    for raw_line in markdown.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            flush_paragraph(&mut paragraph, &mut plain_lines);
        } else if is_plain_heading(line) {
            flush_paragraph(&mut paragraph, &mut plain_lines);
            plain_lines.push(PdfLine::Heading {
                text: line.to_string(),
                level: 2,
            });
        } else {
            paragraph.push(line);
        }
    }
    flush_paragraph(&mut paragraph, &mut plain_lines);
    plain_lines
}

/// Estimate Helvetica/Arial text width closely enough for safe wrapping. Wide characters are
/// deliberately over-estimated so text never crosses the right margin when a system font is absent.
fn estimated_text_width_mm(text: &str, font_size_pt: f32) -> f32 {
    let units: f32 = text
        .chars()
        .map(|character| match character {
            'i' | 'l' | 'I' | 'j' | 't' | 'f' | ' ' | '.' | ',' | ':' | ';' | '!' | '|' => 0.30,
            'm' | 'w' | 'M' | 'W' | '@' | '%' => 0.90,
            _ if character.is_ascii_uppercase() => 0.68,
            _ => 0.55,
        })
        .sum();
    units * font_size_pt * 0.352_778
}

fn wrap_pdf_text(text: &str, font_size_pt: f32, max_width_mm: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if estimated_text_width_mm(word, font_size_pt) > max_width_mm {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }

            let mut chunk = String::new();
            for character in word.chars() {
                let candidate = format!("{chunk}{character}");
                if !chunk.is_empty()
                    && estimated_text_width_mm(&candidate, font_size_pt) > max_width_mm
                {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(character);
            }
            current = chunk;
            continue;
        }

        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && estimated_text_width_mm(&candidate, font_size_pt) > max_width_mm {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_pdf_text(text: &str, font_size_pt: f32, max_width_mm: f32) -> String {
    if estimated_text_width_mm(text, font_size_pt) <= max_width_mm {
        return text.to_string();
    }

    let ellipsis = "...";
    let mut fitted = String::new();
    for character in text.chars() {
        let candidate = format!("{fitted}{character}{ellipsis}");
        if estimated_text_width_mm(&candidate, font_size_pt) > max_width_mm {
            break;
        }
        fitted.push(character);
    }
    fitted.push_str(ellipsis);
    fitted
}

fn generate_pdf_bytes(report: Report, patient: Patient) -> Result<Vec<u8>, AppError> {
    use printpdf::{Color, Line, LinePoint, Op, PdfPage, PdfSaveOptions, Point, Pt, Rgb, TextItem};

    let generated_at = NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&report.generated_at, "%Y-%m-%dT%H:%M:%S%.f"))
        .map(|dt| dt.format("%d.%m.%Y").to_string())
        .unwrap_or_else(|_| report.generated_at.clone());

    let dob = NaiveDate::parse_from_str(&patient.date_of_birth, "%Y-%m-%d")
        .map(|d| d.format("%d.%m.%Y").to_string())
        .unwrap_or_else(|_| patient.date_of_birth.clone());

    let document_title = format_report_type(&report.report_type);
    let patient_name = format!("{} {}", patient.first_name, patient.last_name);
    let mut doc = PdfDocument::new(&document_title);
    doc.metadata.info.document_title = document_title.clone();
    doc.metadata.info.creator = "RamDoc".to_string();
    doc.metadata.info.producer = "RamDoc".to_string();

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

    let dark = Color::Rgb(Rgb::new(0.12, 0.15, 0.19, None));
    let muted = Color::Rgb(Rgb::new(0.38, 0.42, 0.47, None));
    let accent = Color::Rgb(Rgb::new(0.08, 0.32, 0.42, None));
    let rule = Color::Rgb(Rgb::new(0.75, 0.79, 0.82, None));

    let text_ops =
        |text: String, size: f32, x: Mm, y: Mm, fh: &PdfFontHandle, color: &Color| -> Vec<Op> {
            vec![
                Op::StartTextSection,
                Op::SetFillColor { col: color.clone() },
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
    let left = Mm(24.0);
    let right = Mm(186.0);
    let content_width = right.0 - left.0;
    let body_line_height = Mm(5.15);
    let bottom = 24.0;

    let mut all_ops: Vec<Op> = Vec::new();
    let mut pages: Vec<PdfPage> = Vec::new();
    let mut y = Mm(274.0);

    let flush_page = |ops: Vec<Op>, pages: &mut Vec<PdfPage>| {
        pages.push(PdfPage::new(page_w, page_h, ops));
    };

    let add_rule = |ops: &mut Vec<Op>, y: Mm| {
        ops.extend([
            Op::SetOutlineColor { col: rule.clone() },
            Op::SetOutlineThickness { pt: Pt(0.6) },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point::new(left, y),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(right, y),
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
        ]);
    };

    let add_continuation_header = |ops: &mut Vec<Op>| {
        ops.extend(text_ops(
            document_title.clone(),
            8.5,
            left,
            Mm(279.0),
            &font_bold,
            &muted,
        ));
        let header_name = truncate_pdf_text(&patient_name, 8.5, content_width / 2.0);
        let name_width = estimated_text_width_mm(&header_name, 8.5);
        ops.extend(text_ops(
            header_name,
            8.5,
            Mm((right.0 - name_width).max(left.0)),
            Mm(279.0),
            &font,
            &muted,
        ));
        add_rule(ops, Mm(274.0));
    };

    let emit_wrapped = |text: &str,
                        size: f32,
                        fh: &PdfFontHandle,
                        color: &Color,
                        all_ops: &mut Vec<Op>,
                        pages: &mut Vec<PdfPage>,
                        y: &mut Mm| {
        for line in wrap_pdf_text(text, size, content_width) {
            if y.0 < bottom + body_line_height.0 {
                flush_page(std::mem::take(all_ops), pages);
                add_continuation_header(all_ops);
                *y = Mm(265.0);
            }
            all_ops.extend(text_ops(line, size, left, *y, fh, color));
            *y -= body_line_height;
        }
    };

    all_ops.extend(text_ops(
        document_title.clone(),
        19.0,
        left,
        y,
        &font_bold,
        &accent,
    ));
    let date_label = format!("Erstellt am {generated_at}");
    let date_width = estimated_text_width_mm(&date_label, 9.0);
    all_ops.extend(text_ops(
        date_label,
        9.0,
        Mm((right.0 - date_width).max(left.0)),
        Mm(274.0),
        &font,
        &muted,
    ));
    add_rule(&mut all_ops, Mm(265.0));

    y = Mm(255.0);
    emit_wrapped(
        &format!("Patientin/Patient: {patient_name}"),
        10.5,
        &font_bold,
        &dark,
        &mut all_ops,
        &mut pages,
        &mut y,
    );
    let identifiers = format!("Geburtsdatum: {dob}    AHV-Nummer: {}", patient.ahv_number);
    all_ops.extend(text_ops(identifiers, 9.5, left, y, &font, &muted));
    y -= Mm(9.0);

    for line in markdown_to_pdf_lines(&report.content) {
        match line {
            PdfLine::Heading { text, level } => {
                let size = match level {
                    1 => 14.0,
                    2 => 12.0,
                    _ => 11.0,
                };
                if y.0 < bottom + 18.0 {
                    flush_page(std::mem::take(&mut all_ops), &mut pages);
                    add_continuation_header(&mut all_ops);
                    y = Mm(265.0);
                }
                y -= Mm(2.2);
                emit_wrapped(
                    &text,
                    size,
                    &font_bold,
                    &accent,
                    &mut all_ops,
                    &mut pages,
                    &mut y,
                );
                y -= Mm(1.3);
            }
            PdfLine::Body(text) => {
                emit_wrapped(&text, 10.5, &font, &dark, &mut all_ops, &mut pages, &mut y);
            }
            PdfLine::Separator | PdfLine::Blank => {
                y -= Mm(2.6);
            }
        }
    }

    flush_page(all_ops, &mut pages);
    let page_count = pages.len();
    for (index, page) in pages.iter_mut().enumerate() {
        add_rule(&mut page.ops, Mm(17.0));
        let footer = format!("Seite {} von {}", index + 1, page_count);
        let footer_width = estimated_text_width_mm(&footer, 8.0);
        page.ops.extend(text_ops(
            footer,
            8.0,
            Mm((right.0 - footer_width).max(left.0)),
            Mm(11.0),
            &font,
            &muted,
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report(content: String) -> Report {
        Report {
            id: "report-1".to_string(),
            patient_id: "patient-1".to_string(),
            report_type: "Ueberweisungsschreiben".to_string(),
            content,
            generated_at: "2026-08-27 10:15:00".to_string(),
            model_name: Some("test-model".to_string()),
            prompt_hash: None,
            session_ids: None,
            created_at: "2026-08-27 10:15:00".to_string(),
        }
    }

    fn sample_patient() -> Patient {
        Patient {
            id: "patient-1".to_string(),
            ahv_number: "756.1234.5678.97".to_string(),
            first_name: "Erika".to_string(),
            last_name: "Muster".to_string(),
            date_of_birth: "1980-02-03".to_string(),
            gender: None,
            address: None,
            phone: None,
            email: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: "2026-01-01 00:00:00".to_string(),
        }
    }

    #[test]
    fn recognizes_plain_text_clinical_headings() {
        let lines = markdown_to_pdf_lines(
            "Betreff: Mitbeurteilung\n\nSehr geehrte Frau Kollegin\n\n\
             Überweisungsgrund und Fragestellung\n\nBitte um diagnostische Mitbeurteilung.\n\n\
             Aktuelle Medikation\n\nSertralin 100 mg morgens.",
        );

        assert!(lines.contains(&PdfLine::Heading {
            text: "Betreff: Mitbeurteilung".to_string(),
            level: 2,
        }));
        assert!(lines.contains(&PdfLine::Heading {
            text: "Überweisungsgrund und Fragestellung".to_string(),
            level: 2,
        }));
        assert!(lines.contains(&PdfLine::Body(
            "Bitte um diagnostische Mitbeurteilung.".to_string()
        )));
    }

    #[test]
    fn recognizes_plain_headings_alongside_markdown_headings() {
        let lines = markdown_to_pdf_lines(
            "## Überweisungsgrund und Fragestellung\n\nBitte um Mitbeurteilung.\n\n\
             Aktuelle Medikation\n\nSertralin 100 mg morgens.",
        );

        assert!(lines.contains(&PdfLine::Heading {
            text: "Überweisungsgrund und Fragestellung".to_string(),
            level: 2,
        }));
        assert!(lines.contains(&PdfLine::Heading {
            text: "Aktuelle Medikation".to_string(),
            level: 2,
        }));
    }

    #[test]
    fn wraps_text_to_the_available_width() {
        let wrapped = wrap_pdf_text(
            "Klinisch relevante Überweisung mit ausführlicher psychiatrischer Beurteilung",
            10.5,
            55.0,
        );

        assert!(wrapped.len() > 1);
        assert!(wrapped
            .iter()
            .all(|line| estimated_text_width_mm(line, 10.5) <= 55.0));
    }

    #[test]
    fn wraps_long_medical_compounds_without_crossing_the_margin() {
        let wrapped = wrap_pdf_text(
            "Psychopharmakotherapieunverträglichkeitsbeurteilungsanfrage",
            10.5,
            32.0,
        );

        assert!(wrapped.len() > 1);
        assert!(wrapped
            .iter()
            .all(|line| estimated_text_width_mm(line, 10.5) <= 32.0));
    }

    #[test]
    fn truncates_long_continuation_header_names_to_the_available_width() {
        let fitted = truncate_pdf_text(
            "Erika Ausserordentlichlangerzusammengesetzterfamilienname-Muster",
            8.5,
            55.0,
        );

        assert!(fitted.ends_with("..."));
        assert!(estimated_text_width_mm(&fitted, 8.5) <= 55.0);
    }

    #[test]
    fn generated_pdf_contains_header_body_and_footer() {
        let report = sample_report(
            "Betreff: Mitbeurteilung\n\nSehr geehrte Frau Kollegin\n\n\
             Überweisungsgrund und Fragestellung\n\nBitte um diagnostische Mitbeurteilung.\n\n\
             Mit freundlichen Grüssen"
                .to_string(),
        );
        let bytes = generate_pdf_bytes(report, sample_patient()).unwrap();

        assert!(bytes.starts_with(b"%PDF-"));
        let extracted = pdf_extract::extract_text_from_mem(&bytes).unwrap();
        assert!(extracted.contains("Überweisungsschreiben"));
        assert!(extracted.contains("Erika Muster"));
        assert!(extracted.contains("diagnostische Mitbeurteilung"));
        assert!(extracted.contains("Seite 1 von 1"));
    }

    #[test]
    fn generated_pdf_repeats_context_and_page_numbers_after_page_breaks() {
        let content = (1..=45)
            .map(|index| {
                format!(
                    "Klinischer Verlauf {index}: Anhaltende Symptomatik mit relevanter \
                     funktioneller Beeinträchtigung; die dokumentierte Behandlung wird fortgeführt."
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let bytes = generate_pdf_bytes(sample_report(content), sample_patient()).unwrap();
        let pages = pdf_extract::extract_text_from_mem_by_pages(&bytes).unwrap();

        assert!(pages.len() > 1);
        for (index, page) in pages.iter().enumerate() {
            assert!(page.contains("Überweisungsschreiben"));
            assert!(page.contains("Erika Muster"));
            assert!(page.contains(&format!("Seite {} von {}", index + 1, pages.len())));
        }
    }
}

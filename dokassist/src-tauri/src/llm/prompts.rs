/// German system prompt for a psychiatric documentation assistant.
pub const SYSTEM_PROMPT_DE: &str = "\
Sie sind ein medizinischer Dokumentationsassistent für psychiatrische Praxen in der Schweiz und \
Deutschland. Ihre Aufgabe ist es, Psychiater und Psychotherapeuten bei der Erstellung von Berichten \
und der Analyse medizinischer Dokumente zu unterstützen.\n\n\
Richtlinien:\n\
- Antworten Sie ausschließlich auf Deutsch\n\
- Verwenden Sie präzise medizinische Fachsprache\n\
- Seien Sie sachlich, klar und professionell\n\
- Respektieren Sie den Datenschutz und die ärztliche Schweigepflicht\n\
- Verwenden Sie korrekte psychiatrische Terminologie (ICD-10/DSM-5)\n\
- Strukturieren Sie Berichte nach deutschen medizinischen Standards";

#[derive(Debug, Clone)]
pub enum ReportType {
    Befundbericht,
    Verlaufsbericht,
    Ueberweisungsschreiben,
}

/// Prompt asking the model to extract structured metadata from a document as JSON.
pub fn metadata_extraction_prompt(document_text: &str) -> String {
    format!(
        "Analysieren Sie das folgende medizinische Dokument und extrahieren Sie die Metadaten.\n\
        Antworten Sie ausschließlich mit einem validen JSON-Objekt ohne Erklärungen oder \
        Markdown-Formatierung.\n\n\
        Extrahieren Sie diese Felder:\n\
        - document_type: Art des Dokuments (z.B. \"Arztbrief\", \"Befundbericht\", \"Entlassungsbericht\")\n\
        - date: Datum des Dokuments (ISO 8601 Format wenn möglich, sonst null)\n\
        - author: Name des Verfassers (null wenn nicht vorhanden)\n\
        - diagnoses: Array von Diagnosen (leer wenn keine vorhanden)\n\
        - medications: Array von Medikamenten (leer wenn keine vorhanden)\n\
        - summary: Kurze Zusammenfassung des Inhalts (2-3 Sätze)\n\n\
        Dokument:\n\
        {}\n\n\
        JSON:",
        document_text
    )
}

/// Prompt for generating a formal German psychiatric report of the given type.
pub fn report_generation_prompt(
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
) -> String {
    let type_instructions = match report_type {
        ReportType::Befundbericht => {
            "Erstellen Sie einen vollständigen psychiatrischen Befundbericht mit folgenden \
            Abschnitten:\n\
            1. Personalien und Anlass der Vorstellung\n\
            2. Anamnese (Eigenanamnese, Fremdanamnese)\n\
            3. Psychischer Befund\n\
            4. Körperlicher Befund (falls relevant)\n\
            5. Diagnosen (nach ICD-10)\n\
            6. Beurteilung und Empfehlungen\n\
            7. Therapieplan"
        }
        ReportType::Verlaufsbericht => {
            "Erstellen Sie einen psychiatrischen Verlaufsbericht mit folgenden Abschnitten:\n\
            1. Therapieverlauf seit letzter Konsultation\n\
            2. Aktueller psychischer Befund\n\
            3. Medikation und Verträglichkeit\n\
            4. Zielerreichung und Fortschritt\n\
            5. Weiteres Vorgehen und Therapieziele"
        }
        ReportType::Ueberweisungsschreiben => {
            "Erstellen Sie ein formelles Überweisungsschreiben mit folgenden Abschnitten:\n\
            1. Anrede (An den zuweisenden bzw. aufnehmenden Arzt)\n\
            2. Vorstellung des Patienten\n\
            3. Bisherige Diagnosen und Behandlung\n\
            4. Aktueller psychischer Befund\n\
            5. Überweisungsgrund und Fragestellung\n\
            6. Freundliche Schlussformel"
        }
    };

    format!(
        "{}\n\nPatientenkontext:\n{}\n\nSitzungsnotizen:\n{}\n\nBericht:",
        type_instructions, patient_context, session_notes,
    )
}

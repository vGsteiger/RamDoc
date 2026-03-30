/// German system prompt for a psychiatric documentation assistant.
pub const SYSTEM_PROMPT_DE: &str = "\
Sie sind ein medizinischer Dokumentationsassistent für psychiatrische Praxen in der Schweiz und \
Deutschland. Ihre Aufgabe ist es, Psychiater und Psychotherapeuten bei der Erstellung von Berichten \
und der Analyse medizinischer Dokumente zu unterstützen.\n\n\
Richtlinien:\n\
- Antworten Sie ausschliesslich auf Deutsch wie in der Schweiz geschrieben\n\
- Verwenden Sie präzise medizinische Fachsprache\n\
- Seien Sie sachlich, klar und professionell\n\
- Respektieren Sie den Datenschutz und die ärztliche Schweigepflicht\n\
- Verwenden Sie korrekte psychiatrische Terminologie (ICD-10/DSM-5)\n\
- Strukturieren Sie Berichte nach deutschen medizinischen Standards";

/// French system prompt for a psychiatric documentation assistant.
pub const SYSTEM_PROMPT_FR: &str = "\
Vous êtes un assistant de documentation médicale pour les cabinets psychiatriques en Suisse et en France. \
Votre tâche est d'aider les psychiatres et les psychothérapeutes dans la rédaction de rapports et l'analyse \
de documents médicaux.\n\n\
Directives:\n\
- Répondez exclusivement en français\n\
- Utilisez un langage médical précis\n\
- Soyez factuel, clair et professionnel\n\
- Respectez la protection des données et le secret médical\n\
- Utilisez la terminologie psychiatrique correcte (CIM-10/DSM-5)\n\
- Structurez les rapports selon les normes médicales françaises et suisses";

#[derive(Debug, Clone)]
pub enum ReportType {
    Befundbericht,
    Verlaufsbericht,
    Ueberweisungsschreiben,
}

#[derive(Debug, Clone)]
pub enum LetterType {
    Referral,
    InsuranceAuthorization,
    TherapyExtension,
}

/// Prompt asking the model to extract structured metadata from a document as JSON.
///
/// # Security
/// `document_text` is sanitized with `sanitize_for_prompt()` and enclosed in
/// `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
pub fn metadata_extraction_prompt(document_text: &str) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_text = sanitize_for_prompt(document_text);

    let instruction = "Analysieren Sie das folgende medizinische Dokument und extrahieren Sie die Metadaten.\n\
        Antworten Sie ausschliesslich mit einem validen JSON-Objekt ohne Erklärungen oder \
        Markdown-Formatierung.\n\n\
        Extrahieren Sie diese Felder:\n\
        - document_type: Art des Dokuments (z.B. \"Arztbrief\", \"Befundbericht\", \"Entlassungsbericht\")\n\
        - date: Datum des Dokuments (ISO 8601 Format wenn möglich, sonst null)\n\
        - author: Name des Verfassers (null wenn nicht vorhanden)\n\
        - diagnoses: Array von Diagnosen (leer wenn keine vorhanden)\n\
        - medications: Array von Medikamenten (leer wenn keine vorhanden)\n\
        - summary: Kurze Zusammenfassung des Inhalts (2-3 Sätze)";

    let delimited = build_delimited_prompt(instruction, &safe_text);
    format!("{delimited}\nJSON:")
}

/// Prompt for generating a formal German psychiatric report of the given type.
///
/// # Security
/// Both `patient_context` and `session_notes` are sanitized with `sanitize_for_prompt()`
/// and enclosed in `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
pub fn report_generation_prompt(
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_context = sanitize_for_prompt(patient_context);
    let safe_notes = sanitize_for_prompt(session_notes);

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

    let combined_data =
        format!("Patientenkontext:\n{safe_context}\n\nSitzungsnotizen:\n{safe_notes}");
    let delimited = build_delimited_prompt(type_instructions, &combined_data);
    format!("{delimited}\nBericht:")
}

/// Prompt for generating a structured clinical session summary.
///
/// # Security
/// Both `patient_context` and `session_notes` are sanitized with `sanitize_for_prompt()`
/// and enclosed in `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
pub fn session_summary_prompt(patient_context: &str, session_notes: &str) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_context = sanitize_for_prompt(patient_context);
    let safe_notes = sanitize_for_prompt(session_notes);

    let instructions = "Erstellen Sie eine strukturierte klinische Zusammenfassung der Sitzung mit folgenden Abschnitten:\n\
        1. Vorstellungsgrund / Anliegen\n\
        2. Psychischer Zustand (Stimmung, Antrieb, Affekt, formales und inhaltliches Denken)\n\
        3. Interventionen und therapeutisches Vorgehen\n\
        4. Weiteres Vorgehen und Plan\n\n\
        Wichtig:\n\
        - Verwenden Sie nur Informationen aus den bereitgestellten Notizen\n\
        - Erfinden Sie keine Details, die nicht in den Notizen enthalten sind\n\
        - Schreiben Sie in präziser medizinischer Fachsprache\n\
        - Strukturieren Sie die Zusammenfassung mit klaren Überschriften\n\
        - Halten Sie sich an professionelle psychiatrische Dokumentationsstandards";

    let combined_data =
        format!("Patientenkontext:\n{safe_context}\n\nSitzungsnotizen:\n{safe_notes}");
    let delimited = build_delimited_prompt(instructions, &combined_data);
    format!("{delimited}\nZusammenfassung:")
}

/// Prompt for generating a formal letter (referral, insurance authorization, or therapy extension).
///
/// # Security
/// All parameters are sanitized with `sanitize_for_prompt()` and enclosed in
/// `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
pub fn letter_generation_prompt(
    letter_type: LetterType,
    language: &str,
    patient_context: &str,
    clinical_summary: &str,
    recipient_name: Option<&str>,
) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_context = sanitize_for_prompt(patient_context);
    let safe_summary = sanitize_for_prompt(clinical_summary);

    let (type_instructions, greeting) = match (letter_type, language) {
        (LetterType::Referral, "de") => (
            "Erstellen Sie ein formelles Zuweisungsschreiben mit folgenden Abschnitten:\n\
            1. Betreff (z.B. \"Zuweisung zur psychiatrischen/psychotherapeutischen Behandlung\")\n\
            2. Anrede (An den zuweisenden bzw. aufnehmenden Arzt)\n\
            3. Vorstellung des Patienten mit AHV-Nummer und Geburtsdatum\n\
            4. Bisherige Diagnosen und psychiatrischer Befund\n\
            5. Durchgeführte Behandlungen und aktuelle Medikation\n\
            6. Zuweisungsgrund und Fragestellung\n\
            7. Freundliche Schlussformel mit Dank",
            "Sehr geehrte",
        ),
        (LetterType::Referral, "fr") => (
            "Créez une lettre de référence formelle avec les sections suivantes:\n\
            1. Objet (par exemple \"Référence pour traitement psychiatrique/psychothérapeutique\")\n\
            2. Salutation (Au médecin référent ou destinataire)\n\
            3. Présentation du patient avec numéro AVS et date de naissance\n\
            4. Diagnostics antérieurs et état psychiatrique actuel\n\
            5. Traitements effectués et médication actuelle\n\
            6. Raison de la référence et questions\n\
            7. Formule de politesse finale avec remerciements",
            "Madame, Monsieur",
        ),
        (LetterType::InsuranceAuthorization, "de") => (
            "Erstellen Sie ein Kostengutsprache-Gesuch mit folgenden Abschnitten:\n\
            1. Betreff (\"Gesuch um Kostengutsprache für psychiatrisch-psychotherapeutische Behandlung\")\n\
            2. Anrede\n\
            3. Patientenangaben (Name, AHV-Nummer, Geburtsdatum, Versicherung)\n\
            4. Diagnosen nach ICD-10 mit Kodierung\n\
            5. Krankheitsverlauf und bisherige Behandlungen\n\
            6. Begründung der medizinischen Notwendigkeit\n\
            7. Geplante Behandlung (Art, Frequenz, voraussichtliche Dauer)\n\
            8. Behandlungsziele und erwartete Prognose\n\
            9. Höfliche Schlussformel mit der Bitte um Genehmigung",
            "Sehr geehrte Damen und Herren",
        ),
        (LetterType::InsuranceAuthorization, "fr") => (
            "Créez une demande de garantie de prise en charge avec les sections suivantes:\n\
            1. Objet (\"Demande de garantie de prise en charge pour traitement psychiatrique-psychothérapeutique\")\n\
            2. Salutation\n\
            3. Informations sur le patient (nom, numéro AVS, date de naissance, assurance)\n\
            4. Diagnostics selon CIM-10 avec codage\n\
            5. Évolution de la maladie et traitements antérieurs\n\
            6. Justification de la nécessité médicale\n\
            7. Traitement prévu (type, fréquence, durée estimée)\n\
            8. Objectifs thérapeutiques et pronostic attendu\n\
            9. Formule de politesse avec demande d'approbation",
            "Madame, Monsieur",
        ),
        (LetterType::TherapyExtension, "de") => (
            "Erstellen Sie ein Verlängerungsgesuch mit folgenden Abschnitten:\n\
            1. Betreff (\"Gesuch um Verlängerung der Kostengutsprache\")\n\
            2. Anrede\n\
            3. Verweis auf die ursprüngliche Kostengutsprache (Datum, Aktenzeichen falls bekannt)\n\
            4. Patientenangaben (Name, AHV-Nummer, Geburtsdatum)\n\
            5. Bisheriger Therapieverlauf und erreichte Fortschritte\n\
            6. Aktueller psychischer Befund und Symptomatik\n\
            7. Begründung der Notwendigkeit einer Verlängerung\n\
            8. Geplante weitere Behandlung (Frequenz, voraussichtliche Dauer)\n\
            9. Therapieziele für die Verlängerungsperiode\n\
            10. Höfliche Schlussformel mit der Bitte um Genehmigung",
            "Sehr geehrte Damen und Herren",
        ),
        (LetterType::TherapyExtension, "fr") => (
            "Créez une demande de prolongation avec les sections suivantes:\n\
            1. Objet (\"Demande de prolongation de la garantie de prise en charge\")\n\
            2. Salutation\n\
            3. Référence à la garantie originale (date, numéro de dossier si connu)\n\
            4. Informations sur le patient (nom, numéro AVS, date de naissance)\n\
            5. Évolution thérapeutique et progrès réalisés\n\
            6. État psychiatrique actuel et symptomatologie\n\
            7. Justification de la nécessité d'une prolongation\n\
            8. Traitement ultérieur prévu (fréquence, durée estimée)\n\
            9. Objectifs thérapeutiques pour la période de prolongation\n\
            10. Formule de politesse avec demande d'approbation",
            "Madame, Monsieur",
        ),
        _ => (
            "Create a formal letter based on the patient context and clinical summary.",
            "Dear Sir or Madam",
        ),
    };

    let recipient_greeting = if let Some(name) = recipient_name {
        let safe_name = sanitize_for_prompt(name);
        format!("{greeting} {safe_name}")
    } else {
        greeting.to_string()
    };

    let (context_label, summary_label, recipient_label) = match language {
        "de" => (
            "Patientenkontext",
            "Klinische Zusammenfassung",
            "Empfänger-Anrede",
        ),
        "fr" => (
            "Contexte du patient",
            "Résumé clinique",
            "Salutation au destinataire",
        ),
        _ => ("Patient context", "Clinical summary", "Recipient greeting"),
    };

    let combined_data = format!(
        "{context_label}:\n{safe_context}\n\n{summary_label}:\n{safe_summary}\n\n{recipient_label}: {recipient_greeting}"
    );

    let delimited = build_delimited_prompt(type_instructions, &combined_data);

    match language {
        "de" => format!("{delimited}\n\nBrief:"),
        "fr" => format!("{delimited}\n\nLettre:"),
        _ => format!("{delimited}\n\nLetter:"),
    }
}

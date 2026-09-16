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
- Strukturieren Sie Berichte nach deutschen medizinischen Standards\n\
- Verwenden Sie KEIN Markdown, KEINE Sterne, KEINE Rauten und KEINE Listen mit Bindestrichen. Kurze Abschnittsüberschriften als eigene Zeile sind erlaubt";

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
- Structurez les rapports selon les normes médicales françaises et suisses\n\
- N'utilisez PAS de Markdown, PAS d'astérisques, PAS de dièses et PAS de listes à tirets. De courts intertitres sur une ligne séparée sont autorisés";

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
/// All inputs are sanitized with `sanitize_for_prompt()` and enclosed in
/// `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
pub fn report_generation_prompt(
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
    additional_context: Option<&str>,
    instructions: Option<&str>,
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
            "Verfassen Sie ein versandfertiges psychiatrisches Überweisungsschreiben an eine \
            ärztliche oder psychotherapeutische Fachperson. Das Schreiben soll die klinische \
            Entscheidung der empfangenden Fachperson unterstützen, nicht die gesamte Akte \
            nacherzählen.\n\n\
            Verwenden Sie diese Reihenfolge:\n\
            1. Passender Betreff mit Überweisungsziel; keine Überschrift 'Überweisungsschreiben'\n\
            2. Persönliche Anrede, sofern ein Empfänger genannt ist, sonst neutrale Anrede\n\
            3. Einleitung in ein bis zwei Sätzen: Patient, aktueller Anlass und gewünschte Übernahme, \
            Mitbeurteilung oder Behandlung\n\
            4. Überweisungsgrund und konkrete Fragestellung; dieser Abschnitt muss früh und eindeutig sein\n\
            5. Klinisch relevante Anamnese und aktueller Befund, problemorientiert und ohne irrelevante Details\n\
            6. Diagnosen mit ICD-10-Codes, aber nur wenn sie in den Quelldaten vorkommen\n\
            7. Bisheriger Verlauf, relevante Behandlungen und Wirkung beziehungsweise Verträglichkeit\n\
            8. Aktuelle Medikation mit Dosierung und Einnahmeschema, falls vorhanden\n\
            9. Kurze Beurteilung, erbetenes Vorgehen und höfliche Schlussformel\n\n\
            Qualitätsregeln:\n\
            - Verwenden Sie ausschliesslich Angaben aus den bereitgestellten Daten und zusätzlichen Vorgaben.\n\
            - Erfinden Sie keine Befunde, Diagnosen, Risiken, Behandlungen, Namen, Adressen oder Daten.\n\
            - Erweitern oder konkretisieren Sie dokumentierte Symptome nicht. Erfinden Sie insbesondere keine \
              Dauer, Schweregrade, Funktionsbereiche, direkten Zitate oder Beispiele.\n\
            - Leiten Sie Alter, Geschlecht oder Anrede nicht aus Namen oder Geburtsdatum ab. Fehlt eine passende \
              Angabe, formulieren Sie neutral.\n\
            - Fehlt eine Information, lassen Sie sie weg; schreiben Sie keine Platzhalter und keine \
              Formulierungen wie 'nicht angegeben'.\n\
            - Unterscheiden Sie aktuelle Tatsachen, anamnestische Angaben und klinische Einschätzungen.\n\
            - Priorisieren Sie Informationen nach Relevanz für Überweisungsgrund und Fragestellung.\n\
            - Vermeiden Sie Wiederholungen, administrative Meta-Kommentare und eine separate Patientenstammdatenliste; \
              die Stammdaten werden im Dokumentkopf dargestellt.\n\
            - Verwenden Sie durchgehend Schweizer Rechtschreibung mit 'ss', insbesondere 'Grüsse', niemals 'Grüße'.\n\
            - Geben Sie ausschliesslich den fertigen Brieftext aus, ohne Entwurfsvermerke, eckige Klammern, \
              Unterschriftsplatzhalter oder Erläuterungen an die schreibende Person.\n\
            - Schreiben Sie präzise, kollegial und gut lesbar. Nutzen Sie kurze Absätze. Verwenden Sie die \
              folgenden unnummerierten Abschnittsüberschriften, sofern passende Quelldaten vorhanden sind: \
              'Überweisungsgrund und Fragestellung', 'Relevante Anamnese und aktueller Befund', 'Diagnosen', \
              'Bisheriger Verlauf und Behandlung', 'Aktuelle Medikation' sowie \
              'Beurteilung und erbetenes Vorgehen'. Zielumfang: ungefähr eine bis zwei Seiten."
        }
    };

    let mut combined_data =
        format!("Patientenkontext:\n{safe_context}\n\nSitzungsnotizen:\n{safe_notes}");

    if let Some(ctx) = additional_context.filter(|s| !s.is_empty()) {
        let safe_ctx = sanitize_for_prompt(ctx);
        combined_data.push_str(&format!("\n\nZusatzdokument:\n{safe_ctx}"));
    }

    let full_instructions = if let Some(instr) = instructions.filter(|s| !s.is_empty()) {
        let safe_instr = sanitize_for_prompt(instr);
        format!("{type_instructions}\n\nZusätzliche Vorgaben:\n{safe_instr}")
    } else {
        type_instructions.to_string()
    };

    let delimited = build_delimited_prompt(&full_instructions, &combined_data);
    format!(
        "{delimited}\nWICHTIG: Nur reiner Text, kein Markdown, keine Sterne und keine Rauten. \
        Abschnittsüberschriften stehen ohne Nummerierung oder Satzzeichen auf einer eigenen Zeile.\n\
        FINALER SELBSTCHECK VOR DER AUSGABE:\n\
        - Jede klinische Aussage muss unmittelbar auf eine konkrete Angabe in den klinischen Daten zurückgehen.\n\
        - Nicht vorhandene Angaben vollständig weglassen; weder das Fehlen erwähnen noch Details ergänzen.\n\
        - Keine eckigen Klammern, Unterschriftsfelder, Platzhalter oder Hinweise an die schreibende Person.\n\
        - Schweizer Rechtschreibung mit ss verwenden; das Zeichen ß darf nicht vorkommen.\n\
        - Nur den fertigen Brief ausgeben.\nBericht:"
    )
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
    format!("{delimited}\nWICHTIG: Nur reiner Fliesstext, kein Markdown, keine Sterne, keine Rauten.\nZusammenfassung:")
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

    let no_fmt = "WICHTIG: Nur reiner Fliesstext, kein Markdown, keine Sterne, keine Rauten.";
    match language {
        "de" => format!("{delimited}\n\n{no_fmt}\nBrief:"),
        "fr" => format!("{delimited}\n\n{no_fmt}\nLettre:"),
        _ => format!("{delimited}\n\n{no_fmt}\nLetter:"),
    }
}

/// Prompt for condensing long patient context + session notes into a compact clinical summary.
/// Used as a pre-pass when the full input would overflow the model's context window.
///
/// # Security
/// Both inputs are sanitized with `sanitize_for_prompt()` before insertion.
pub fn context_summarization_prompt(patient_context: &str, session_notes: &str) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_context = sanitize_for_prompt(patient_context);
    let safe_notes = sanitize_for_prompt(session_notes);

    let instructions = "Fassen Sie die folgenden Patientendaten und Sitzungsnotizen in einer kompakten klinischen Zusammenfassung zusammen.\n\
        Behalten Sie vollständig bei:\n\
        - Alle aktiven ICD-10-Diagnosen mit Datum und Beschreibung\n\
        - Alle aktuellen Medikamente mit Dosierung und Häufigkeit\n\
        - Den vollständigen Inhalt der aktuellen Sitzungsnotizen\n\
        - Wesentliche Verlaufsinformationen der letzten Sitzungen (komprimiert)\n\
        - Aktuelle Behandlungsziele\n\
        Vermeiden Sie Wiederholungen. Schreiben Sie präzise medizinische Fachsprache. \
        Halten Sie die Zusammenfassung so kurz wie möglich, ohne klinisch relevante Informationen zu verlieren.";

    let combined =
        format!("Patientenkontext:\n{safe_context}\n\nAktuelle Sitzungsnotizen:\n{safe_notes}");
    format!(
        "{}\nZusammenfassung:",
        build_delimited_prompt(instructions, &combined)
    )
}

/// Prompt for continuing a report that was cut off due to context window limits.
/// `partial_output` should be the last portion (≤ 800 chars) of the incomplete report.
///
/// # Security
/// `partial_output` is sanitized with `sanitize_for_prompt()` before insertion.
pub fn continuation_prompt(partial_output: &str) -> String {
    use super::sanitize::sanitize_for_prompt;

    let safe_tail = sanitize_for_prompt(partial_output);

    format!(
        "Ein psychiatrischer Bericht wurde begonnen, konnte aber aufgrund von Platzbeschränkungen \
        nicht vollständig generiert werden. Hier ist das Ende des bisher erstellten Texts:\n\n\
        [...]\n{safe_tail}\n\n\
        Bitte vervollständigen Sie den Bericht. Fahren Sie direkt an der Stelle fort, wo der Text \
        endet. Wiederholen Sie nichts, was bereits geschrieben wurde. Schreiben Sie nur den \
        fehlenden Rest des Berichts.\nFortsetzung:"
    )
}

/// Prompt for answering questions about a patient's history using RAG.
///
/// # Security
/// `patient_context` is sanitized with `sanitize_for_prompt()` and enclosed in
/// `===== CLINICAL DATA START/END =====` delimiter markers before insertion.
/// `question` is also sanitized to prevent injection attacks.
pub fn patient_history_query_prompt(patient_context: &str, question: &str) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_context = sanitize_for_prompt(patient_context);
    let safe_question = sanitize_for_prompt(question);

    let instructions = format!(
        "Beantworten Sie die folgende Frage basierend ausschließlich auf den bereitgestellten \
        Patientenakten.\n\n\
        Wichtige Regeln:\n\
        - Antworten Sie NUR mit Informationen aus den bereitgestellten Akten\n\
        - Zitieren Sie immer Sitzungsdaten oder Datumsangaben, wenn Sie sich auf spezifische Ereignisse beziehen\n\
        - Wenn die Informationen nicht in den Akten enthalten sind, antworten Sie mit: \
        \"Diese Information ist in den vorliegenden Akten nicht dokumentiert.\"\n\
        - Erfinden Sie keine Details oder Informationen\n\
        - Seien Sie präzise und verwenden Sie medizinische Fachsprache\n\
        - Wenn Sie Trends oder Veränderungen beschreiben, nennen Sie die spezifischen Daten und Werte\n\n\
        Frage: {safe_question}"
    );

    let delimited = build_delimited_prompt(&instructions, &safe_context);
    format!("{delimited}\nAntwort:")
}

/// Prompt for answering a patient-history question from an assembled evidence
/// block (see `llm::evidence`).
///
/// Unlike [`patient_history_query_prompt`], every evidence line carries a
/// citation marker, so the model is required to cite instead of merely
/// mentioning dates. Those markers are what
/// `llm::evidence::audit_answer` traces back to source revisions.
///
/// # Security
/// `evidence` is produced by the assembler, which sanitises each unit's text
/// with `sanitize_for_prompt()` as it renders it — re-sanitising the whole block
/// here would hit the per-field length cap and truncate the evidence. `question`
/// is sanitized before insertion.
pub fn evidence_query_prompt(evidence: &str, question: &str) -> String {
    use super::sanitize::{build_delimited_prompt, sanitize_for_prompt};

    let safe_question = sanitize_for_prompt(question);

    let instructions = format!(
        "Beantworten Sie die folgende Frage ausschliesslich anhand der unten aufgeführten \
        Evidenzauszüge.\n\n\
        Wichtige Regeln:\n\
        - Jeder Auszug beginnt mit einer Quellenangabe in der Form [E1 | Datum | Quelle | Revision]\n\
        - Belegen Sie jede Aussage mit der passenden Kennung in eckigen Klammern, z. B. [E3]\n\
        - Verwenden Sie ausschliesslich Informationen aus den Auszügen; ergänzen Sie nichts\n\
        - Übernehmen Sie Dosierungen, Daten, Negationen und Unsicherheitsangaben wörtlich\n\
        - Fehlt die Information, antworten Sie: \
        \"Diese Information ist in den vorliegenden Auszügen nicht dokumentiert.\"\n\
        - Der Abschnitt \"NICHT ENTHALTEN, ABER VORHANDEN\" listet nur Fundstellen ohne Inhalt; \
        leiten Sie daraus keine Aussagen ab, sondern verweisen Sie darauf\n\n\
        Frage: {safe_question}"
    );

    let delimited = build_delimited_prompt(&instructions, evidence);
    format!("{delimited}\nAntwort mit Quellenangaben:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referral_prompt_prioritizes_question_and_forbids_invention() {
        let prompt = report_generation_prompt(
            ReportType::Ueberweisungsschreiben,
            "Name: Erika Muster\nDiagnose: F33.1",
            "Anhaltende depressive Symptomatik",
            None,
            Some("Überweisungsgrund und konkrete Fragestellung: Bitte um Mitbeurteilung"),
        );

        assert!(prompt.contains("früh und eindeutig"));
        assert!(prompt.contains("Erfinden Sie keine Befunde"));
        assert!(prompt.contains("Priorisieren Sie Informationen nach Relevanz"));
        assert!(prompt.contains("Erweitern oder konkretisieren Sie dokumentierte Symptome nicht"));
        assert!(prompt.contains("Unterschriftsplatzhalter"));
        assert!(prompt.contains("Schweizer Rechtschreibung"));
        assert!(prompt.contains("Bitte um Mitbeurteilung"));
        assert!(prompt.contains("Abschnittsüberschriften stehen ohne Nummerierung"));
    }

    #[test]
    fn referral_prompt_keeps_clinical_data_delimited() {
        let prompt = report_generation_prompt(
            ReportType::Ueberweisungsschreiben,
            "Patientenkontext",
            "Sitzungsnotizen",
            Some("Zusatzdokument"),
            None,
        );

        assert_eq!(prompt.matches("===== CLINICAL DATA START =====").count(), 1);
        assert_eq!(prompt.matches("===== CLINICAL DATA END =====").count(), 1);
        assert!(prompt.contains("Zusatzdokument"));
    }
}

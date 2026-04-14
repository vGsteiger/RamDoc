use crate::error::AppError;
use crate::models::{diagnosis, medication, outcome_score, patient, session, treatment_plan};
use rusqlite::Connection;

/// Assembles comprehensive patient history context for RAG queries
pub fn assemble_patient_context(
    conn: &Connection,
    patient_id: &str,
) -> Result<String, AppError> {
    let patient = patient::get_patient(conn, patient_id)?;

    // Fetch patient data with appropriate limits to avoid loading excessive data
    // Most recent 20 sessions, all active diagnoses and current medications
    let sessions = session::list_sessions_for_patient(conn, patient_id, 20, 0)?;
    let diagnoses = diagnosis::list_diagnoses_for_patient(conn, patient_id, 100, 0)?;
    let medications = medication::list_medications_for_patient(conn, patient_id, 100, 0)?;
    let outcome_scores = outcome_score::list_scores_for_patient(conn, patient_id, 100, 0)?;
    let treatment_plans = treatment_plan::list_treatment_plans_for_patient(conn, patient_id, 20, 0)?;

    // Format patient context as structured text
    let mut context = String::new();

    // Patient demographics
    context.push_str("===== PATIENT INFORMATION =====\n");
    context.push_str(&format!(
        "Name: {} {}\n",
        patient.first_name, patient.last_name
    ));
    context.push_str(&format!("Date of Birth: {}\n", patient.date_of_birth));
    context.push_str(&format!("AHV Number: {}\n", patient.ahv_number));
    if let Some(gender) = &patient.gender {
        context.push_str(&format!("Gender: {}\n", gender));
    }
    if let Some(insurance) = &patient.insurance {
        context.push_str(&format!("Insurance: {}\n", insurance));
    }
    if let Some(gp_name) = &patient.gp_name {
        context.push_str(&format!("General Practitioner: {}\n", gp_name));
    }
    if let Some(notes) = &patient.notes {
        if !notes.trim().is_empty() {
            context.push_str(&format!("\nPatient Notes:\n{}\n", notes));
        }
    }
    context.push_str("\n");

    // Active diagnoses
    context.push_str("===== DIAGNOSES =====\n");
    let active_diagnoses: Vec<_> = diagnoses
        .iter()
        .filter(|d| d.status.to_lowercase() == "active")
        .collect();
    let resolved_diagnoses: Vec<_> = diagnoses
        .iter()
        .filter(|d| d.status.to_lowercase() != "active")
        .collect();

    if !active_diagnoses.is_empty() {
        context.push_str("Active Diagnoses:\n");
        for diagnosis in &active_diagnoses {
            context.push_str(&format!(
                "- {} ({}): {} (diagnosed: {})\n",
                diagnosis.icd10_code,
                diagnosis.status,
                diagnosis.description,
                diagnosis.diagnosed_date
            ));
            if let Some(notes) = &diagnosis.notes {
                if !notes.trim().is_empty() {
                    context.push_str(&format!("  Notes: {}\n", notes));
                }
            }
        }
    }

    if !resolved_diagnoses.is_empty() {
        context.push_str("\nResolved/Inactive Diagnoses:\n");
        for diagnosis in &resolved_diagnoses {
            let resolved_info = diagnosis
                .resolved_date
                .as_ref()
                .map(|d| format!(" (resolved: {})", d))
                .unwrap_or_default();
            context.push_str(&format!(
                "- {} ({}): {} (diagnosed: {}{})\n",
                diagnosis.icd10_code,
                diagnosis.status,
                diagnosis.description,
                diagnosis.diagnosed_date,
                resolved_info
            ));
        }
    }

    if diagnoses.is_empty() {
        context.push_str("No diagnoses recorded.\n");
    }
    context.push_str("\n");

    // Current and past medications
    context.push_str("===== MEDICATIONS =====\n");
    let current_medications: Vec<_> = medications
        .iter()
        .filter(|m| m.end_date.is_none())
        .collect();
    let past_medications: Vec<_> = medications
        .iter()
        .filter(|m| m.end_date.is_some())
        .collect();

    if !current_medications.is_empty() {
        context.push_str("Current Medications:\n");
        for medication in &current_medications {
            context.push_str(&format!(
                "- {}: {} {} (started: {})\n",
                medication.substance, medication.dosage, medication.frequency, medication.start_date
            ));
            if let Some(notes) = &medication.notes {
                if !notes.trim().is_empty() {
                    context.push_str(&format!("  Notes: {}\n", notes));
                }
            }
        }
    }

    if !past_medications.is_empty() {
        context.push_str("\nPast Medications:\n");
        for medication in &past_medications {
            let end_date = medication
                .end_date
                .as_ref()
                .map(|d| d.as_str())
                .unwrap_or("unknown");
            context.push_str(&format!(
                "- {}: {} {} ({} to {})\n",
                medication.substance,
                medication.dosage,
                medication.frequency,
                medication.start_date,
                end_date
            ));
        }
    }

    if medications.is_empty() {
        context.push_str("No medications recorded.\n");
    }
    context.push_str("\n");

    // Treatment plans
    context.push_str("===== TREATMENT PLANS =====\n");
    if !treatment_plans.is_empty() {
        for plan in &treatment_plans {
            context.push_str(&format!(
                "Plan: {} (status: {}, start: {}",
                plan.title, plan.status, plan.start_date
            ));
            if let Some(end_date) = &plan.end_date {
                context.push_str(&format!(", end: {}", end_date));
            }
            context.push_str(")\n");

            if let Some(description) = &plan.description {
                if !description.trim().is_empty() {
                    context.push_str(&format!("  Description: {}\n", description));
                }
            }

            // Fetch goals for this treatment plan
            if let Ok(goals) =
                treatment_plan::list_treatment_goals_for_plan(conn, &plan.id, u32::MAX, 0)
            {
                if !goals.is_empty() {
                    context.push_str("  Goals:\n");
                    for goal in goals {
                        let target = goal
                            .target_date
                            .as_ref()
                            .map(|d| format!(" (target: {})", d))
                            .unwrap_or_default();
                        context.push_str(&format!(
                            "    - {} (status: {}{})\n",
                            goal.description, goal.status, target
                        ));
                    }
                }
            }

            // Fetch interventions for this treatment plan
            if let Ok(interventions) =
                treatment_plan::list_treatment_interventions_for_plan(conn, &plan.id, u32::MAX, 0)
            {
                if !interventions.is_empty() {
                    context.push_str("  Interventions:\n");
                    for intervention in interventions {
                        let frequency = intervention
                            .frequency
                            .as_ref()
                            .map(|f| format!(" ({})", f))
                            .unwrap_or_default();
                        context.push_str(&format!(
                            "    - {} (type: {}{}\n",
                            intervention.description,
                            intervention.r#type,
                            frequency
                        ));
                    }
                }
            }
            context.push_str("\n");
        }
    } else {
        context.push_str("No treatment plans recorded.\n\n");
    }

    // Outcome scores
    context.push_str("===== OUTCOME SCORES =====\n");
    if !outcome_scores.is_empty() {
        // Group by scale type
        let mut phq9_scores: Vec<_> = outcome_scores
            .iter()
            .filter(|s| s.scale_type == "PHQ-9")
            .collect();
        let mut gad7_scores: Vec<_> = outcome_scores
            .iter()
            .filter(|s| s.scale_type == "GAD-7")
            .collect();
        let mut bdi_scores: Vec<_> = outcome_scores
            .iter()
            .filter(|s| s.scale_type == "BDI-II")
            .collect();

        // Sort by date (most recent first)
        phq9_scores.sort_by(|a, b| b.administered_at.cmp(&a.administered_at));
        gad7_scores.sort_by(|a, b| b.administered_at.cmp(&a.administered_at));
        bdi_scores.sort_by(|a, b| b.administered_at.cmp(&a.administered_at));

        if !phq9_scores.is_empty() {
            context.push_str("PHQ-9 Scores (Depression):\n");
            for score in phq9_scores.iter().take(10) {
                // Show last 10 scores
                let interpretation = score.interpretation.as_deref().unwrap_or("no interpretation");
                context.push_str(&format!(
                    "  {}: {} ({} - {})\n",
                    score.administered_at, score.score, score.scale_type, interpretation
                ));
            }
            if phq9_scores.len() > 10 {
                context.push_str(&format!("  ... and {} older scores\n", phq9_scores.len() - 10));
            }
        }

        if !gad7_scores.is_empty() {
            context.push_str("GAD-7 Scores (Anxiety):\n");
            for score in gad7_scores.iter().take(10) {
                let interpretation = score.interpretation.as_deref().unwrap_or("no interpretation");
                context.push_str(&format!(
                    "  {}: {} ({} - {})\n",
                    score.administered_at, score.score, score.scale_type, interpretation
                ));
            }
            if gad7_scores.len() > 10 {
                context.push_str(&format!("  ... and {} older scores\n", gad7_scores.len() - 10));
            }
        }

        if !bdi_scores.is_empty() {
            context.push_str("BDI-II Scores (Depression):\n");
            for score in bdi_scores.iter().take(10) {
                let interpretation = score.interpretation.as_deref().unwrap_or("no interpretation");
                context.push_str(&format!(
                    "  {}: {} ({} - {})\n",
                    score.administered_at, score.score, score.scale_type, interpretation
                ));
            }
            if bdi_scores.len() > 10 {
                context.push_str(&format!("  ... and {} older scores\n", bdi_scores.len() - 10));
            }
        }
    } else {
        context.push_str("No outcome scores recorded.\n");
    }
    context.push_str("\n");

    // Session history (most recent first, limited by query)
    context.push_str("===== SESSION HISTORY =====\n");
    if !sessions.is_empty() {
        context.push_str(&format!(
            "Showing {} most recent sessions:\n\n",
            sessions.len()
        ));

        for session in &sessions {
            context.push_str(&format!("--- Session: {} ---\n", session.session_date));
            context.push_str(&format!("Type: {}\n", session.session_type));
            if let Some(duration) = session.duration_minutes {
                context.push_str(&format!("Duration: {} minutes\n", duration));
            }

            if let Some(notes) = &session.notes {
                if !notes.trim().is_empty() {
                    context.push_str("Notes:\n");
                    context.push_str(notes);
                    context.push_str("\n");
                }
            }

            if let Some(summary) = &session.clinical_summary {
                if !summary.trim().is_empty() {
                    context.push_str("Clinical Summary:\n");
                    context.push_str(summary);
                    context.push_str("\n");
                }
            }

            context.push_str("\n");
        }
    } else {
        context.push_str("No sessions recorded.\n");
    }

    Ok(context)
}

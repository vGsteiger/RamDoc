use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::{diagnosis, medication, outcome_score, patient, session};
use crate::models::fhir::{
    Address, Annotation, Bundle, CodeableConcept, Condition, ContactPoint, Dosage, HumanName,
    Identifier, MedicationStatement, Observation, Patient as FhirPatient, Period, Reference,
    Resource,
};
use crate::state::AppState;
use rusqlite::Connection;
use tauri::State;

/// Export patient data as a FHIR R4 Bundle (type: document)
#[tauri::command]
pub async fn export_fhir_bundle(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<String, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Get patient data
    let patient = patient::get_patient(&conn, &patient_id)?;

    // Build FHIR resources
    let mut resources: Vec<Resource> = Vec::new();

    // 1. Patient resource
    let fhir_patient = build_fhir_patient(&patient);
    resources.push(Resource::Patient(fhir_patient));

    // 2. Condition resources (from diagnoses)
    let diagnoses = diagnosis::list_diagnoses_for_patient(&conn, &patient_id, u32::MAX, 0)?;
    for diag in diagnoses {
        let condition = build_fhir_condition(&diag, &patient_id);
        resources.push(Resource::Condition(condition));
    }

    // 3. MedicationStatement resources
    let medications = medication::list_medications_for_patient(&conn, &patient_id, u32::MAX, 0)?;
    for med in medications {
        let med_statement = build_fhir_medication_statement(&med, &patient_id);
        resources.push(Resource::MedicationStatement(med_statement));
    }

    // 4. Observation resources (from outcome scores)
    let observations = build_fhir_observations_from_scores(&conn, &patient_id)?;
    for obs in observations {
        resources.push(Resource::Observation(obs));
    }

    // Create FHIR Bundle
    let bundle = Bundle::new_document(resources);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| AppError::Validation(format!("Failed to serialize FHIR bundle: {}", e)))?;

    // Log audit event
    audit::log(
        &conn,
        AuditAction::Export,
        "patient_fhir_bundle",
        Some(&patient_id),
        Some("FHIR R4 Bundle export"),
    )?;

    Ok(json)
}

/// Build FHIR Patient resource from internal Patient model
fn build_fhir_patient(patient: &patient::Patient) -> FhirPatient {
    // Build identifiers - Swiss AHV number
    let identifiers = vec![Identifier::swiss_ahv(&patient.ahv_number)];

    // Build name
    let name = vec![HumanName {
        family: Some(patient.last_name.clone()),
        given: Some(vec![patient.first_name.clone()]),
    }];

    // Build address if available
    let address = patient.address.as_ref().map(|addr| {
        vec![Address {
            text: Some(addr.clone()),
        }]
    });

    // Build telecom (contact points)
    let mut telecom = Vec::new();
    if let Some(ref phone) = patient.phone {
        telecom.push(ContactPoint {
            system: "phone".to_string(),
            value: Some(phone.clone()),
        });
    }
    if let Some(ref email) = patient.email {
        telecom.push(ContactPoint {
            system: "email".to_string(),
            value: Some(email.clone()),
        });
    }
    let telecom = if telecom.is_empty() {
        None
    } else {
        Some(telecom)
    };

    // Map gender
    let gender = patient.gender.as_ref().map(|g| {
        match g.to_lowercase().as_str() {
            "male" | "m" => "male",
            "female" | "f" => "female",
            "other" => "other",
            _ => "unknown",
        }
        .to_string()
    });

    FhirPatient {
        resource_type: "Patient".to_string(),
        id: patient.id.clone(),
        identifier: identifiers,
        name,
        birth_date: patient.date_of_birth.clone(),
        gender,
        address,
        telecom,
    }
}

/// Build FHIR Condition resource from internal Diagnosis model
fn build_fhir_condition(diag: &diagnosis::Diagnosis, patient_id: &str) -> Condition {
    let subject = Reference {
        reference: format!("Patient/{}", patient_id),
    };

    let code = CodeableConcept::from_icd10(&diag.icd10_code, &diag.description);

    let clinical_status = CodeableConcept::clinical_status(&diag.status);

    let onset_date_time = Some(diag.diagnosed_date.clone());

    let abatement_date_time = diag.resolved_date.clone();

    let note = diag.notes.as_ref().map(|n| {
        vec![Annotation {
            text: n.clone(),
        }]
    });

    Condition {
        resource_type: "Condition".to_string(),
        id: diag.id.clone(),
        subject,
        code,
        clinical_status,
        onset_date_time,
        abatement_date_time,
        note,
    }
}

/// Build FHIR MedicationStatement resource from internal Medication model
fn build_fhir_medication_statement(
    med: &medication::Medication,
    patient_id: &str,
) -> MedicationStatement {
    let subject = Reference {
        reference: format!("Patient/{}", patient_id),
    };

    // Determine status based on end_date
    let status = if med.end_date.is_some() {
        "completed"
    } else {
        "active"
    }
    .to_string();

    let medication_codeable_concept = CodeableConcept::from_text(&med.substance);

    let effective_period = Some(Period {
        start: Some(med.start_date.clone()),
        end: med.end_date.clone(),
    });

    // Build dosage information
    let dosage_text = format!("{} {}", med.dosage, med.frequency);
    let dosage = Some(vec![Dosage {
        text: Some(dosage_text),
    }]);

    let note = med.notes.as_ref().map(|n| {
        vec![Annotation {
            text: n.clone(),
        }]
    });

    MedicationStatement {
        resource_type: "MedicationStatement".to_string(),
        id: med.id.clone(),
        subject,
        status,
        medication_codeable_concept,
        effective_period,
        dosage,
        note,
    }
}

/// Build FHIR Observation resources from outcome scores
fn build_fhir_observations_from_scores(
    conn: &Connection,
    patient_id: &str,
) -> Result<Vec<Observation>, AppError> {
    let mut observations = Vec::new();

    // Get all sessions for the patient
    let sessions = session::list_sessions_for_patient(conn, patient_id, u32::MAX, 0)?;

    for sess in sessions {
        // Get outcome scores for this session
        let scores = outcome_score::list_scores_for_session(conn, &sess.id, u32::MAX, 0)?;

        for score in scores {
            let obs = build_fhir_observation(&score, patient_id);
            observations.push(obs);
        }
    }

    Ok(observations)
}

/// Build a single FHIR Observation from an outcome score
fn build_fhir_observation(score: &outcome_score::OutcomeScore, patient_id: &str) -> Observation {
    let subject = Reference {
        reference: format!("Patient/{}", patient_id),
    };

    // Map scale type to LOINC code
    let (loinc_code, loinc_display) = match score.scale_type.as_str() {
        "PHQ-9" => ("44249-1", "PHQ-9 total score"),
        "GAD-7" => ("69737-5", "GAD-7 total score"),
        "BDI-II" => ("89209-1", "Beck Depression Inventory II total score"), // Common LOINC for BDI-II
        _ => ("", "Unknown scale"),
    };

    let code = CodeableConcept::from_loinc(loinc_code, loinc_display);

    let value_integer = Some(score.score);

    let effective_date_time = Some(score.administered_at.clone());

    let interpretation = score.interpretation.as_ref().map(|interp| {
        vec![CodeableConcept::interpretation(interp)]
    });

    let note = score.notes.as_ref().map(|n| {
        vec![Annotation {
            text: n.clone(),
        }]
    });

    Observation {
        resource_type: "Observation".to_string(),
        id: score.id.clone(),
        subject,
        status: "final".to_string(),
        code,
        value_integer,
        effective_date_time,
        interpretation,
        note,
    }
}

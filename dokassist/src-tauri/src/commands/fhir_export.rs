use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::fhir::{
    Address, Annotation, Bundle, CodeableConcept, Condition, ContactPoint, Dosage, HumanName,
    Identifier, MedicationStatement, Observation, Patient as FhirPatient, Period, Reference,
    Resource,
};
use crate::models::{diagnosis, medication, outcome_score, patient, session};
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

    let note = diag
        .notes
        .as_ref()
        .map(|n| vec![Annotation { text: n.clone() }]);

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

    let note = med
        .notes
        .as_ref()
        .map(|n| vec![Annotation { text: n.clone() }]);

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

    let interpretation = score
        .interpretation
        .as_ref()
        .map(|interp| vec![CodeableConcept::interpretation(interp)]);

    let note = score
        .notes
        .as_ref()
        .map(|n| vec![Annotation { text: n.clone() }]);

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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use crate::models::patient::CreatePatient;
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let dir = tempdir().unwrap();
        let key = crate::crypto::generate_key();
        let pool = init_db(&dir.path().join("test.db"), &key).unwrap();
        (dir, pool)
    }

    #[test]
    fn test_build_fhir_patient() {
        let patient = patient::Patient {
            id: "test-patient-123".to_string(),
            ahv_number: "756.1234.5678.97".to_string(),
            first_name: "Hans".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-15".to_string(),
            gender: Some("male".to_string()),
            address: Some("Bahnhofstrasse 1, 8001 Zürich".to_string()),
            phone: Some("+41791234567".to_string()),
            email: Some("hans.mueller@example.com".to_string()),
            insurance: Some("CSS".to_string()),
            gp_name: Some("Dr. Schmidt".to_string()),
            gp_address: Some("Arztpraxis, Zürich".to_string()),
            notes: Some("Test notes".to_string()),
            created_at: "2024-01-01T10:00:00Z".to_string(),
            updated_at: "2024-01-01T10:00:00Z".to_string(),
        };

        let fhir_patient = build_fhir_patient(&patient);

        assert_eq!(fhir_patient.resource_type, "Patient");
        assert_eq!(fhir_patient.id, "test-patient-123");
        assert_eq!(fhir_patient.birth_date, "1980-01-15");
        assert_eq!(fhir_patient.gender, Some("male".to_string()));

        // Check identifier
        assert_eq!(fhir_patient.identifier.len(), 1);
        assert_eq!(fhir_patient.identifier[0].value, "756.1234.5678.97");
        assert_eq!(
            fhir_patient.identifier[0].system,
            Some("urn:oid:2.16.756.5.32".to_string())
        );

        // Check name
        assert_eq!(fhir_patient.name.len(), 1);
        assert_eq!(fhir_patient.name[0].family, Some("Müller".to_string()));
        assert_eq!(fhir_patient.name[0].given, Some(vec!["Hans".to_string()]));

        // Check telecom
        let telecom = fhir_patient.telecom.unwrap();
        assert_eq!(telecom.len(), 2);
        assert_eq!(telecom[0].system, "phone");
        assert_eq!(telecom[0].value, Some("+41791234567".to_string()));
        assert_eq!(telecom[1].system, "email");
        assert_eq!(
            telecom[1].value,
            Some("hans.mueller@example.com".to_string())
        );
    }

    #[test]
    fn test_build_fhir_condition() {
        let diagnosis = diagnosis::Diagnosis {
            id: "diag-123".to_string(),
            patient_id: "patient-123".to_string(),
            icd10_code: "F32.1".to_string(),
            description: "Moderate depressive episode".to_string(),
            status: "active".to_string(),
            diagnosed_date: "2024-01-15".to_string(),
            resolved_date: None,
            notes: Some("Patient responding well to treatment".to_string()),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
        };

        let condition = build_fhir_condition(&diagnosis, "patient-123");

        assert_eq!(condition.resource_type, "Condition");
        assert_eq!(condition.id, "diag-123");
        assert_eq!(condition.subject.reference, "Patient/patient-123");

        // Check ICD-10 coding
        let coding = condition.code.coding.as_ref().unwrap();
        assert_eq!(
            coding[0].system,
            Some("http://hl7.org/fhir/sid/icd-10".to_string())
        );
        assert_eq!(coding[0].code, Some("F32.1".to_string()));
        assert_eq!(
            coding[0].display,
            Some("Moderate depressive episode".to_string())
        );

        // Check clinical status
        let clinical_coding = condition.clinical_status.coding.as_ref().unwrap();
        assert_eq!(clinical_coding[0].code, Some("active".to_string()));

        assert_eq!(condition.onset_date_time, Some("2024-01-15".to_string()));
        assert!(condition.abatement_date_time.is_none());
    }

    #[test]
    fn test_build_fhir_medication_statement() {
        let medication = medication::Medication {
            id: "med-123".to_string(),
            patient_id: "patient-123".to_string(),
            substance: "Sertraline".to_string(),
            dosage: "50mg".to_string(),
            frequency: "daily".to_string(),
            start_date: "2024-01-01".to_string(),
            end_date: None,
            notes: Some("Take in the morning".to_string()),
            created_at: "2024-01-01T10:00:00Z".to_string(),
            updated_at: "2024-01-01T10:00:00Z".to_string(),
        };

        let med_statement = build_fhir_medication_statement(&medication, "patient-123");

        assert_eq!(med_statement.resource_type, "MedicationStatement");
        assert_eq!(med_statement.id, "med-123");
        assert_eq!(med_statement.status, "active");
        assert_eq!(
            med_statement.medication_codeable_concept.text,
            Some("Sertraline".to_string())
        );

        // Check effective period
        let period = med_statement.effective_period.as_ref().unwrap();
        assert_eq!(period.start, Some("2024-01-01".to_string()));
        assert!(period.end.is_none());

        // Check dosage
        let dosage = med_statement.dosage.as_ref().unwrap();
        assert_eq!(dosage[0].text, Some("50mg daily".to_string()));
    }

    #[test]
    fn test_build_fhir_observation_phq9() {
        let score = outcome_score::OutcomeScore {
            id: "score-123".to_string(),
            session_id: "session-123".to_string(),
            scale_type: "PHQ-9".to_string(),
            score: 15,
            interpretation: Some("Moderately Severe".to_string()),
            subscores: None,
            administered_at: "2024-01-15T10:00:00Z".to_string(),
            notes: None,
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
        };

        let observation = build_fhir_observation(&score, "patient-123");

        assert_eq!(observation.resource_type, "Observation");
        assert_eq!(observation.id, "score-123");
        assert_eq!(observation.subject.reference, "Patient/patient-123");
        assert_eq!(observation.status, "final");

        // Check LOINC coding for PHQ-9
        let coding = observation.code.coding.as_ref().unwrap();
        assert_eq!(coding[0].system, Some("http://loinc.org".to_string()));
        assert_eq!(coding[0].code, Some("44249-1".to_string()));
        assert_eq!(coding[0].display, Some("PHQ-9 total score".to_string()));

        assert_eq!(observation.value_integer, Some(15));
        assert_eq!(
            observation.effective_date_time,
            Some("2024-01-15T10:00:00Z".to_string())
        );

        // Check interpretation
        let interp = observation.interpretation.as_ref().unwrap();
        assert_eq!(interp[0].text, Some("Moderately Severe".to_string()));
    }

    #[test]
    fn test_build_fhir_observation_gad7() {
        let score = outcome_score::OutcomeScore {
            id: "score-456".to_string(),
            session_id: "session-456".to_string(),
            scale_type: "GAD-7".to_string(),
            score: 10,
            interpretation: Some("Moderate".to_string()),
            subscores: None,
            administered_at: "2024-01-20T10:00:00Z".to_string(),
            notes: None,
            created_at: "2024-01-20T10:00:00Z".to_string(),
            updated_at: "2024-01-20T10:00:00Z".to_string(),
        };

        let observation = build_fhir_observation(&score, "patient-456");

        // Check LOINC coding for GAD-7
        let coding = observation.code.coding.as_ref().unwrap();
        assert_eq!(coding[0].code, Some("69737-5".to_string()));
        assert_eq!(coding[0].display, Some("GAD-7 total score".to_string()));

        assert_eq!(observation.value_integer, Some(10));
    }

    #[test]
    fn test_fhir_bundle_structure() {
        let (_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // Create a test patient
        let patient = patient::create_patient(
            &conn,
            CreatePatient {
                ahv_number: "7561234567897".to_string(),
                first_name: "Test".to_string(),
                last_name: "Patient".to_string(),
                date_of_birth: "1990-01-01".to_string(),
                gender: Some("female".to_string()),
                address: None,
                phone: None,
                email: None,
                insurance: None,
                gp_name: None,
                gp_address: None,
                notes: None,
            },
        )
        .unwrap();

        // Build FHIR patient
        let fhir_patient = build_fhir_patient(&patient);

        // Create a bundle
        let bundle = Bundle::new_document(vec![Resource::Patient(fhir_patient)]);

        assert_eq!(bundle.resource_type, "Bundle");
        assert_eq!(bundle.bundle_type, "document");
        assert_eq!(bundle.entry.len(), 1);

        // Serialize to JSON to ensure it's valid
        let json = serde_json::to_string_pretty(&bundle).unwrap();
        assert!(json.contains("\"resourceType\": \"Bundle\""));
        assert!(json.contains("\"type\": \"document\""));
        assert!(json.contains("\"resourceType\": \"Patient\""));
    }
}

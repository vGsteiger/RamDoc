use serde::{Deserialize, Serialize};

/// FHIR R4 Bundle resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    #[serde(rename = "resourceType")]
    pub resource_type: String, // "Bundle"
    #[serde(rename = "type")]
    pub bundle_type: String, // "document"
    pub entry: Vec<BundleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEntry {
    pub resource: Resource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Resource {
    Patient(Patient),
    Condition(Condition),
    MedicationStatement(MedicationStatement),
    Observation(Observation),
}

/// FHIR R4 Patient resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    #[serde(rename = "resourceType")]
    pub resource_type: String, // "Patient"
    pub id: String,
    pub identifier: Vec<Identifier>,
    pub name: Vec<HumanName>,
    #[serde(rename = "birthDate")]
    pub birth_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<Address>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<ContactPoint>>,
}

/// FHIR R4 Condition resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(rename = "resourceType")]
    pub resource_type: String, // "Condition"
    pub id: String,
    pub subject: Reference,
    pub code: CodeableConcept,
    #[serde(rename = "clinicalStatus")]
    pub clinical_status: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "onsetDateTime")]
    pub onset_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "abatementDateTime")]
    pub abatement_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,
}

/// FHIR R4 MedicationStatement resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicationStatement {
    #[serde(rename = "resourceType")]
    pub resource_type: String, // "MedicationStatement"
    pub id: String,
    pub subject: Reference,
    pub status: String, // "active" | "completed" | "entered-in-error" | "intended" | "stopped" | "on-hold" | "unknown" | "not-taken"
    #[serde(rename = "medicationCodeableConcept")]
    pub medication_codeable_concept: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "effectivePeriod")]
    pub effective_period: Option<Period>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dosage: Option<Vec<Dosage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,
}

/// FHIR R4 Observation resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    #[serde(rename = "resourceType")]
    pub resource_type: String, // "Observation"
    pub id: String,
    pub subject: Reference,
    pub status: String, // "registered" | "preliminary" | "final" | "amended" | "corrected" | "cancelled" | "entered-in-error" | "unknown"
    pub code: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "valueInteger")]
    pub value_integer: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "effectiveDateTime")]
    pub effective_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<CodeableConcept>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,
}

// Common FHIR data types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub identifier_type: Option<CodeableConcept>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPoint {
    pub system: String, // "phone" | "email" | etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub reference: String, // e.g., "Patient/123"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<Coding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dosage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

// Helper functions for creating FHIR resources

impl Bundle {
    pub fn new_document(entries: Vec<Resource>) -> Self {
        Self {
            resource_type: "Bundle".to_string(),
            bundle_type: "document".to_string(),
            entry: entries.into_iter().map(|r| BundleEntry { resource: r }).collect(),
        }
    }
}

impl CodeableConcept {
    /// Create a CodeableConcept from ICD-10 code
    pub fn from_icd10(code: &str, display: &str) -> Self {
        Self {
            coding: Some(vec![Coding {
                system: Some("http://hl7.org/fhir/sid/icd-10".to_string()),
                code: Some(code.to_string()),
                display: Some(display.to_string()),
            }]),
            text: Some(display.to_string()),
        }
    }

    /// Create a CodeableConcept from LOINC code
    pub fn from_loinc(code: &str, display: &str) -> Self {
        Self {
            coding: Some(vec![Coding {
                system: Some("http://loinc.org".to_string()),
                code: Some(code.to_string()),
                display: Some(display.to_string()),
            }]),
            text: Some(display.to_string()),
        }
    }

    /// Create a CodeableConcept for clinical status
    pub fn clinical_status(status: &str) -> Self {
        let code = match status.to_lowercase().as_str() {
            "active" => "active",
            "remission" => "remission",
            "resolved" => "resolved",
            _ => "unknown",
        };
        Self {
            coding: Some(vec![Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/condition-clinical".to_string()),
                code: Some(code.to_string()),
                display: Some(code.to_string()),
            }]),
            text: Some(code.to_string()),
        }
    }

    /// Create a CodeableConcept for interpretation
    pub fn interpretation(text: &str) -> Self {
        Self {
            coding: None,
            text: Some(text.to_string()),
        }
    }

    /// Create a simple text-based CodeableConcept
    pub fn from_text(text: &str) -> Self {
        Self {
            coding: None,
            text: Some(text.to_string()),
        }
    }
}

impl Identifier {
    /// Create a Swiss AHV identifier for EPD
    pub fn swiss_ahv(ahv_number: &str) -> Self {
        Self {
            system: Some("urn:oid:2.16.756.5.32".to_string()), // Swiss EPD identifier system OID
            identifier_type: Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: Some("http://terminology.hl7.org/CodeSystem/v2-0203".to_string()),
                    code: Some("NI".to_string()), // National Person Identifier
                    display: Some("National unique individual identifier".to_string()),
                }]),
                text: Some("AHV Number".to_string()),
            }),
            value: ahv_number.to_string(),
        }
    }
}

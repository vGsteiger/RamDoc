use serde::{Deserialize, Serialize};

/// Practice settings stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeSettings {
    pub practice_name: Option<String>,
    pub practice_address: Option<String>,
    pub practice_phone: Option<String>,
    pub practice_email: Option<String>,
    pub therapist_name: Option<String>,
    pub zsr_number: Option<String>,
    pub canton: Option<String>,
    pub clinical_specialty: Option<String>,
    pub language_preference: String,
    pub onboarding_completed: bool,
}

impl Default for PracticeSettings {
    fn default() -> Self {
        Self {
            practice_name: None,
            practice_address: None,
            practice_phone: None,
            practice_email: None,
            therapist_name: None,
            zsr_number: None,
            canton: None,
            clinical_specialty: None,
            language_preference: "de".to_string(),
            onboarding_completed: false,
        }
    }
}

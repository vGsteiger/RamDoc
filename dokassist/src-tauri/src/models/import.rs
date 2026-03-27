use serde::{Deserialize, Serialize};

/// Column mapping from CSV header to patient field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub csv_header: String,
    pub patient_field: String,
}

/// Warning encountered during CSV parsing or validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvWarning {
    pub row: Option<usize>,
    pub column: Option<String>,
    pub message: String,
}

/// Preview of CSV file before import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvPreview {
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub detected_mappings: Vec<ColumnMapping>,
    pub warnings: Vec<CsvWarning>,
}

/// Result of CSV import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported_count: usize,
    pub failed_count: usize,
    pub warnings: Vec<CsvWarning>,
    pub errors: Vec<CsvWarning>,
}

/// CSV row data mapped to patient fields
#[derive(Debug, Clone, Default)]
pub struct CsvPatientRow {
    pub ahv_number: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
}

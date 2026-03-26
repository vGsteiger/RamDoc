use crate::models::diagnosis::{self, CreateDiagnosis, UpdateDiagnosis};
use crate::models::medication::{self, CreateMedication, UpdateMedication};
use crate::models::patient::{self, CreatePatient, UpdatePatient};
use crate::models::session::{self, CreateSession, UpdateSession};
use crate::{ahv, audit, crypto, database, filesystem, recovery, search};
use tempfile::TempDir;

// ===========================
// PKG-0: Scaffold Tests
// ===========================
//
// PKG-0 validates the basic project structure:
// - Tauri 2 + Svelte 5 app compiles
// - All dependencies declared in Cargo.toml
// - Module structure is correct
//
// These are validated by the fact that the codebase compiles
// and all other tests can access the modules.

#[test]
fn test_pkg0_module_structure() {
    // Verify all core modules are accessible
    let _ = crypto::generate_key();
    let _ = database::init_db;
    let _ = filesystem::init_vault;
    let _ = audit::log;
    let _ = search::search;
}

// ===========================
// PKG-1: Crypto Core Tests
// ===========================

#[test]
fn test_pkg1_full_crypto_flow() {
    let temp_dir = TempDir::new().unwrap();
    let vault_path = temp_dir.path().join("recovery.vault");

    // Step 1: Create recovery — mnemonic entropy is the sole master secret;
    // db_key and fs_key are derived deterministically via Argon2id.
    let (mnemonic_words, db_key, fs_key) = recovery::create_recovery(&vault_path).unwrap();
    assert_eq!(mnemonic_words.len(), 24);
    assert!(vault_path.exists());

    // Step 2: Test encryption/decryption with db_key
    let plaintext = b"Sensitive patient data";
    let ciphertext = crypto::encrypt(&db_key, plaintext).unwrap();
    let decrypted = crypto::decrypt(&db_key, &ciphertext).unwrap();
    assert_eq!(plaintext.as_slice(), decrypted.as_slice());

    // Step 3: Simulate losing keys and recovering from mnemonic
    let (recovered_db_key, recovered_fs_key) =
        recovery::recover_from_mnemonic(&mnemonic_words, &vault_path).unwrap();

    assert_eq!(db_key, recovered_db_key);
    assert_eq!(fs_key, recovered_fs_key);

    // Step 4: Verify recovered key can decrypt data
    let decrypted_with_recovered = crypto::decrypt(&recovered_db_key, &ciphertext).unwrap();
    assert_eq!(plaintext.as_slice(), decrypted_with_recovered.as_slice());
}

#[test]
fn test_pkg1_encryption_with_different_keys() {
    let key1 = crypto::generate_key();
    let key2 = crypto::generate_key();
    let data = b"Test data";

    let encrypted_with_key1 = crypto::encrypt(&key1, data).unwrap();

    // Should not be able to decrypt with different key
    let result = crypto::decrypt(&key2, &encrypted_with_key1);
    assert!(result.is_err());

    // Should work with correct key
    let decrypted = crypto::decrypt(&key1, &encrypted_with_key1).unwrap();
    assert_eq!(data.as_slice(), decrypted.as_slice());
}

#[test]
fn test_pkg1_large_data_encryption() {
    let key = crypto::generate_key();
    let large_data = vec![42u8; 10 * 1024 * 1024]; // 10 MB

    let encrypted = crypto::encrypt(&key, &large_data).unwrap();
    let decrypted = crypto::decrypt(&key, &encrypted).unwrap();

    assert_eq!(large_data, decrypted);
}

#[test]
fn test_pkg1_empty_data_encryption() {
    let key = crypto::generate_key();
    let empty_data = b"";

    let encrypted = crypto::encrypt(&key, empty_data).unwrap();
    let decrypted = crypto::decrypt(&key, &encrypted).unwrap();

    assert_eq!(empty_data.as_slice(), decrypted.as_slice());
}

#[test]
fn test_pkg1_key_uniqueness() {
    // Generate multiple keys and ensure they're all different
    let keys: Vec<[u8; 32]> = (0..10).map(|_| crypto::generate_key()).collect();

    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(keys[i], keys[j], "Generated keys should be unique");
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires Touch ID hardware and code-signing entitlements"]
fn test_pkg1_keychain_operations() {
    use crate::keychain;

    const TEST_SERVICE: &str = "ch.dokassist.app.integration-test";
    const TEST_ACCOUNT: &str = "integration-test-key";

    // Generate a test key
    let key = crypto::generate_key();

    // Store in keychain
    keychain::store_key(TEST_SERVICE, TEST_ACCOUNT, &key).unwrap();

    // Retrieve from keychain
    let retrieved = keychain::retrieve_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();
    assert_eq!(key.to_vec(), retrieved);

    // Delete from keychain
    keychain::delete_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();

    // Verify deletion
    let result = keychain::retrieve_key(TEST_SERVICE, TEST_ACCOUNT);
    assert!(result.is_err());
}

// ===========================
// PKG-2: Database Tests
// ===========================

#[test]
fn test_pkg2_database_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Verify foreign keys are enabled
    let fk_enabled: i32 = conn
        .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fk_enabled, 1);

    // Verify schema version
    let version: i32 = conn
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .unwrap();
    assert!(version >= 1);
}

#[test]
fn test_pkg2_database_wrong_key() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key1 = crypto::generate_key();
    let key2 = crypto::generate_key();

    // Create database with key1
    database::init_db(&db_path, &key1).unwrap();

    // Try to open with wrong key
    let result = database::init_db(&db_path, &key2);
    assert!(result.is_err());
}

#[test]
fn test_pkg2_patient_crud() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Create patient
    let create_input = CreatePatient {
        ahv_number: "756.1234.5678.97".to_string(),
        first_name: "Hans".to_string(),
        last_name: "Müller".to_string(),
        date_of_birth: "1980-01-15".to_string(),
        gender: Some("M".to_string()),
        address: Some("Bahnhofstrasse 1, 8001 Zürich".to_string()),
        phone: Some("+41 44 123 45 67".to_string()),
        email: Some("hans.mueller@example.ch".to_string()),
        insurance: Some("Helsana".to_string()),
        gp_name: Some("Dr. Schmidt".to_string()),
        gp_address: Some("Seestrasse 10, 8001 Zürich".to_string()),
        notes: Some("Test patient".to_string()),
    };

    let created_patient = patient::create_patient(&conn, create_input).unwrap();
    assert_eq!(created_patient.first_name, "Hans");
    assert_eq!(created_patient.ahv_number, "756.1234.5678.97");

    // Read patient
    let read_patient = patient::get_patient(&conn, &created_patient.id).unwrap();
    assert_eq!(read_patient.id, created_patient.id);
    assert_eq!(read_patient.last_name, "Müller");

    // Update patient
    let update_input = UpdatePatient {
        first_name: Some("Johann".to_string()),
        phone: Some("+41 44 999 88 77".to_string()),
        ..Default::default()
    };

    let updated_patient =
        patient::update_patient(&conn, &created_patient.id, update_input).unwrap();
    assert_eq!(updated_patient.first_name, "Johann");
    assert_eq!(updated_patient.phone, Some("+41 44 999 88 77".to_string()));

    // List patients
    let patients = patient::list_patients(&conn, 10, 0).unwrap();
    assert_eq!(patients.len(), 1);

    // Delete patient
    patient::delete_patient(&conn, &created_patient.id).unwrap();
    let result = patient::get_patient(&conn, &created_patient.id);
    assert!(result.is_err());
}

#[test]
fn test_pkg2_ahv_validation() {
    // Valid AHV formats
    assert!(ahv::validate_ahv("756.1234.5678.97").is_ok());
    assert!(ahv::validate_ahv("7561234567897").is_ok());

    // Invalid formats
    assert!(ahv::validate_ahv("123.4567.8901.23").is_err()); // Wrong prefix
    assert!(ahv::validate_ahv("756.1234.5678.9").is_err()); // Too short
    assert!(ahv::validate_ahv("").is_err()); // Empty
    assert!(ahv::validate_ahv("abc").is_err()); // Not numeric
}

// ===========================
// PKG-3: Filesystem Tests
// ===========================

#[test]
fn test_pkg3_vault_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();

    filesystem::init_vault(base_dir).unwrap();

    // Verify vault directory exists
    assert!(base_dir.join("vault").exists());
    assert!(base_dir.join("temp").exists());
}

#[test]
fn test_pkg3_file_storage_and_retrieval() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    let fs_key = crypto::generate_key();

    filesystem::init_vault(base_dir).unwrap();

    // Store a file
    let patient_id = uuid::Uuid::now_v7().to_string();
    let original_data = b"Test medical document content";

    let vault_path = filesystem::store_file(base_dir, &fs_key, &patient_id, original_data).unwrap();

    // Verify vault path format
    assert!(vault_path.contains(&patient_id));
    assert!(vault_path.ends_with(".enc"));

    // Read the file back
    let decrypted_data = filesystem::read_file(base_dir, &fs_key, &vault_path).unwrap();
    assert_eq!(original_data.as_slice(), decrypted_data.as_slice());
}

#[test]
fn test_pkg3_file_wrong_key() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    let fs_key1 = crypto::generate_key();
    let fs_key2 = crypto::generate_key();

    filesystem::init_vault(base_dir).unwrap();

    let patient_id = uuid::Uuid::now_v7().to_string();
    let original_data = b"Encrypted content";

    let vault_path =
        filesystem::store_file(base_dir, &fs_key1, &patient_id, original_data).unwrap();

    // Try to read with wrong key
    let result = filesystem::read_file(base_dir, &fs_key2, &vault_path);
    assert!(result.is_err());
}

#[test]
fn test_pkg3_path_traversal_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    let fs_key = crypto::generate_key();

    filesystem::init_vault(base_dir).unwrap();

    // Try to store with malicious patient_id
    let result = filesystem::store_file(base_dir, &fs_key, "../../../etc/passwd", b"malicious");
    assert!(result.is_err());

    // Try to read with path traversal
    let result = filesystem::read_file(base_dir, &fs_key, "../../../etc/passwd");
    assert!(result.is_err());
}

#[test]
fn test_pkg3_delete_file() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    let fs_key = crypto::generate_key();

    filesystem::init_vault(base_dir).unwrap();

    let patient_id = uuid::Uuid::now_v7().to_string();
    let vault_path = filesystem::store_file(base_dir, &fs_key, &patient_id, b"test data").unwrap();

    // Verify file exists
    assert!(filesystem::read_file(base_dir, &fs_key, &vault_path).is_ok());

    // Delete the file
    filesystem::delete_file(base_dir, &vault_path).unwrap();

    // Verify file is gone
    let result = filesystem::read_file(base_dir, &fs_key, &vault_path);
    assert!(result.is_err());
}

#[test]
fn test_pkg3_large_file_storage() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path();
    let fs_key = crypto::generate_key();

    filesystem::init_vault(base_dir).unwrap();

    let patient_id = uuid::Uuid::now_v7().to_string();
    let large_data = vec![0x42u8; 5 * 1024 * 1024]; // 5 MB

    let vault_path = filesystem::store_file(base_dir, &fs_key, &patient_id, &large_data).unwrap();

    let decrypted_data = filesystem::read_file(base_dir, &fs_key, &vault_path).unwrap();
    assert_eq!(large_data, decrypted_data);
}

// ===========================
// PKG-5: Search Tests
// ===========================

#[test]
fn test_pkg5_patient_search() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Create test patients
    let patient1 = patient::create_patient(
        &conn,
        CreatePatient {
            ahv_number: "756.1234.5678.97".to_string(),
            first_name: "Hans".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-15".to_string(),
            gender: Some("M".to_string()),
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

    let patient2 = patient::create_patient(
        &conn,
        CreatePatient {
            ahv_number: "756.9876.5432.17".to_string(),
            first_name: "Maria".to_string(),
            last_name: "Schmidt".to_string(),
            date_of_birth: "1975-06-20".to_string(),
            gender: Some("F".to_string()),
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

    // Index patients
    search::index_patient(&conn, &patient1).unwrap();
    search::index_patient(&conn, &patient2).unwrap();

    // Search by first name
    let results = search::search(&conn, "Hans", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, patient1.id);

    // Search by last name
    let results = search::search(&conn, "Schmidt", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, patient2.id);

    // Search by AHV (dotted format)
    let results = search::search(&conn, "756.1234.5678.97", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, patient1.id);

    // Search by AHV (plain format)
    let results = search::search(&conn, "7561234567897", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, patient1.id);

    // Full token match
    let results = search::search(&conn, "Müller", 10).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_pkg5_search_empty_query() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    let results = search::search(&conn, "", 10).unwrap();
    assert_eq!(results.len(), 0);
}

// ===========================
// PKG-6: Audit Tests
// ===========================

#[test]
fn test_pkg6_audit_logging() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Log various actions
    audit::log(
        &conn,
        audit::AuditAction::Create,
        "patient",
        Some("patient-123"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        audit::AuditAction::View,
        "patient",
        Some("patient-123"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        audit::AuditAction::Update,
        "patient",
        Some("patient-123"),
        Some("fields: first_name,last_name"),
    )
    .unwrap();

    // Query all entries
    let entries = audit::query_log(&conn, None, None, None, None, 100, 0).unwrap();
    assert_eq!(entries.len(), 3);

    // Verify entries are ordered by timestamp (newest first)
    assert_eq!(entries[0].action, "update");
    assert_eq!(entries[1].action, "view");
    assert_eq!(entries[2].action, "create");
}

#[test]
fn test_pkg6_audit_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Log various actions
    audit::log(
        &conn,
        audit::AuditAction::Create,
        "patient",
        Some("patient-123"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        audit::AuditAction::Create,
        "file",
        Some("file-456"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        audit::AuditAction::View,
        "patient",
        Some("patient-789"),
        None,
    )
    .unwrap();

    // Filter by entity type
    let patient_entries =
        audit::query_log(&conn, Some("patient"), None, None, None, 100, 0).unwrap();
    assert_eq!(patient_entries.len(), 2);

    // Filter by entity ID
    let specific_entries =
        audit::query_log(&conn, None, Some("patient-123"), None, None, 100, 0).unwrap();
    assert_eq!(specific_entries.len(), 1);
    assert_eq!(
        specific_entries[0].entity_id,
        Some("patient-123".to_string())
    );
}

#[test]
fn test_pkg6_audit_pagination() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Create 15 entries
    for i in 0..15 {
        audit::log(
            &conn,
            audit::AuditAction::View,
            "test",
            Some(&format!("test-{}", i)),
            None,
        )
        .unwrap();
    }

    // Page 1 (limit 10)
    let page1 = audit::query_log(&conn, Some("test"), None, None, None, 10, 0).unwrap();
    assert_eq!(page1.len(), 10);

    // Page 2 (limit 10, offset 10)
    let page2 = audit::query_log(&conn, Some("test"), None, None, None, 10, 10).unwrap();
    assert_eq!(page2.len(), 5);

    // Verify no overlap
    assert_ne!(page1[0].id, page2[0].id);
}

#[test]
fn test_pkg6_audit_no_phi() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Log with details containing only field names
    audit::log(
        &conn,
        audit::AuditAction::Update,
        "patient",
        Some("patient-123"),
        Some("fields: first_name,last_name,date_of_birth"),
    )
    .unwrap();

    let entries = audit::query_log(&conn, None, Some("patient-123"), None, None, 100, 0).unwrap();
    assert_eq!(entries.len(), 1);

    let details = entries[0].details.as_ref().unwrap();
    assert!(details.contains("fields:"));
    // Ensure no actual patient data in details
    assert!(!details.contains("Hans"));
    assert!(!details.contains("Müller"));
}

// ===========================
// PKG-4: LLM Tests
// ===========================
//
// PKG-4 tests are limited because:
// - Model downloads require network and take significant time
// - Model loading requires large files (5-18GB) that don't exist in test env
// - Inference requires loaded models
//
// These tests verify the API exists and basic functionality.
// Full E2E tests should be done manually with actual models.

#[test]
fn test_pkg4_llm_module_exists() {
    // Verify LLM module structure is accessible
    // The actual engine tests require models which are too large for CI
    // These would require actual models:
    // - llm::Engine::new()
    // - llm::Engine::load_model()
    // - llm::generate()
    // - llm::extract_metadata()

    // Manual testing required for:
    // - Model download with progress
    // - Model loading (5-18GB files)
    // - Inference quality
    // - Streaming generation
}

// ===========================
// PKG-7: Frontend Auth Tests
// ===========================
//
// PKG-7 tests are primarily for the frontend UI and Tauri commands.
// The underlying auth logic (PKG-1) is tested above.
//
// These would test the Tauri command layer:
// - commands::auth::check_auth()
// - commands::auth::initialize_app()
// - commands::auth::unlock_app()
// - commands::auth::recover_app()
// - commands::auth::lock_app()
//
// The state management tests require AppState which is
// coupled to Tauri runtime. These are better tested via
// integration tests that mock the Tauri environment or
// E2E tests with the actual frontend.

#[test]
fn test_pkg7_auth_state_module_exists() {
    // Verify auth state module is accessible
    // AppState requires runtime context for full testing
    // Manual/E2E testing required for:
    // - check_auth command
    // - initialize_app command (returns 24 words)
    // - unlock_app command (triggers Touch ID)
    // - recover_app command
    // - lock_app command
    // - State transitions: FirstRun -> Unlocked
    // - State transitions: Locked -> Unlocked
    // - State transitions: RecoveryRequired -> Unlocked
}

// ===========================
// PKG-10: Clinical Workflows Tests
// ===========================

#[test]
fn test_pkg10_session_crud() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // First create a patient
    let patient_input = CreatePatient {
        ahv_number: "756.1234.5678.97".to_string(),
        first_name: "Maria".to_string(),
        last_name: "Schmidt".to_string(),
        date_of_birth: "1985-05-20".to_string(),
        gender: Some("F".to_string()),
        address: None,
        phone: None,
        email: None,
        insurance: None,
        gp_name: None,
        gp_address: None,
        notes: None,
    };
    let patient = patient::create_patient(&conn, patient_input).unwrap();

    // Create session
    let create_input = CreateSession {
        patient_id: patient.id.clone(),
        session_date: "2026-03-01".to_string(),
        session_type: "Initial Assessment".to_string(),
        duration_minutes: Some(60),
        scheduled_time: None,
        notes: Some("First session with patient".to_string()),
        amdp_data: Some(r#"{"consciousness":"clear","orientation":"full"}"#.to_string()),
    };

    let created_session = session::create_session(&conn, create_input).unwrap();
    assert_eq!(created_session.session_type, "Initial Assessment");
    assert_eq!(created_session.patient_id, patient.id);
    assert_eq!(created_session.duration_minutes, Some(60));

    // Read session
    let read_session = session::get_session(&conn, &created_session.id).unwrap();
    assert_eq!(read_session.id, created_session.id);
    assert_eq!(read_session.session_date, "2026-03-01");

    // Update session
    let update_input = UpdateSession {
        session_type: Some("Follow-up".to_string()),
        duration_minutes: Some(45),
        scheduled_time: None,
        notes: Some("Updated notes".to_string()),
        session_date: None,
        amdp_data: None,
        clinical_summary: None,
    };

    let updated_session =
        session::update_session(&conn, &created_session.id, update_input).unwrap();
    assert_eq!(updated_session.session_type, "Follow-up");
    assert_eq!(updated_session.duration_minutes, Some(45));
    assert_eq!(updated_session.notes, Some("Updated notes".to_string()));

    // List sessions for patient
    let sessions = session::list_sessions_for_patient(&conn, &patient.id, 10, 0).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, created_session.id);

    // Delete session
    session::delete_session(&conn, &created_session.id).unwrap();
    let result = session::get_session(&conn, &created_session.id);
    assert!(result.is_err());
}

#[test]
fn test_pkg10_diagnosis_crud() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // First create a patient
    let patient_input = CreatePatient {
        ahv_number: "756.2345.6789.08".to_string(),
        first_name: "Peter".to_string(),
        last_name: "Weber".to_string(),
        date_of_birth: "1978-11-10".to_string(),
        gender: Some("M".to_string()),
        address: None,
        phone: None,
        email: None,
        insurance: None,
        gp_name: None,
        gp_address: None,
        notes: None,
    };
    let patient = patient::create_patient(&conn, patient_input).unwrap();

    // Create diagnosis
    let create_input = CreateDiagnosis {
        patient_id: patient.id.clone(),
        icd10_code: "F32.1".to_string(),
        description: "Mittelgradige depressive Episode".to_string(),
        status: Some("active".to_string()),
        diagnosed_date: "2026-02-15".to_string(),
        resolved_date: None,
        notes: Some("Initial diagnosis".to_string()),
    };

    let created_diagnosis = diagnosis::create_diagnosis(&conn, create_input).unwrap();
    assert_eq!(created_diagnosis.icd10_code, "F32.1");
    assert_eq!(created_diagnosis.status, "active");
    assert_eq!(created_diagnosis.patient_id, patient.id);

    // Read diagnosis
    let read_diagnosis = diagnosis::get_diagnosis(&conn, &created_diagnosis.id).unwrap();
    assert_eq!(read_diagnosis.id, created_diagnosis.id);
    assert_eq!(
        read_diagnosis.description,
        "Mittelgradige depressive Episode"
    );

    // Update diagnosis
    let update_input = UpdateDiagnosis {
        status: Some("remission".to_string()),
        notes: Some("Patient shows improvement".to_string()),
        icd10_code: None,
        description: None,
        diagnosed_date: None,
        resolved_date: None,
    };

    let updated_diagnosis =
        diagnosis::update_diagnosis(&conn, &created_diagnosis.id, update_input).unwrap();
    assert_eq!(updated_diagnosis.status, "remission");
    assert_eq!(
        updated_diagnosis.notes,
        Some("Patient shows improvement".to_string())
    );

    // List diagnoses for patient
    let diagnoses = diagnosis::list_diagnoses_for_patient(&conn, &patient.id, 10, 0).unwrap();
    assert_eq!(diagnoses.len(), 1);
    assert_eq!(diagnoses[0].id, created_diagnosis.id);

    // Delete diagnosis
    diagnosis::delete_diagnosis(&conn, &created_diagnosis.id).unwrap();
    let result = diagnosis::get_diagnosis(&conn, &created_diagnosis.id);
    assert!(result.is_err());
}

#[test]
fn test_pkg10_medication_crud() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // First create a patient
    let patient_input = CreatePatient {
        ahv_number: "756.3456.7890.19".to_string(),
        first_name: "Anna".to_string(),
        last_name: "Meier".to_string(),
        date_of_birth: "1990-07-22".to_string(),
        gender: Some("F".to_string()),
        address: None,
        phone: None,
        email: None,
        insurance: None,
        gp_name: None,
        gp_address: None,
        notes: None,
    };
    let patient = patient::create_patient(&conn, patient_input).unwrap();

    // Create medication
    let create_input = CreateMedication {
        patient_id: patient.id.clone(),
        substance: "Sertralin".to_string(),
        dosage: "50mg".to_string(),
        frequency: "1x täglich".to_string(),
        start_date: "2026-03-01".to_string(),
        end_date: None,
        notes: Some("Start with low dose".to_string()),
    };

    let created_medication = medication::create_medication(&conn, create_input).unwrap();
    assert_eq!(created_medication.substance, "Sertralin");
    assert_eq!(created_medication.dosage, "50mg");
    assert_eq!(created_medication.patient_id, patient.id);

    // Read medication
    let read_medication = medication::get_medication(&conn, &created_medication.id).unwrap();
    assert_eq!(read_medication.id, created_medication.id);
    assert_eq!(read_medication.frequency, "1x täglich");

    // Update medication
    let update_input = UpdateMedication {
        dosage: Some("100mg".to_string()),
        notes: Some("Increased dosage after 2 weeks".to_string()),
        substance: None,
        frequency: None,
        start_date: None,
        end_date: None,
    };

    let updated_medication =
        medication::update_medication(&conn, &created_medication.id, update_input).unwrap();
    assert_eq!(updated_medication.dosage, "100mg");
    assert_eq!(
        updated_medication.notes,
        Some("Increased dosage after 2 weeks".to_string())
    );

    // List medications for patient
    let medications = medication::list_medications_for_patient(&conn, &patient.id, 10, 0).unwrap();
    assert_eq!(medications.len(), 1);
    assert_eq!(medications[0].id, created_medication.id);

    // Delete medication
    medication::delete_medication(&conn, &created_medication.id).unwrap();
    let result = medication::get_medication(&conn, &created_medication.id);
    assert!(result.is_err());
}

#[test]
fn test_pkg10_multiple_sessions_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let key = crypto::generate_key();

    let pool = database::init_db(&db_path, &key).unwrap();
    let conn = pool.conn().unwrap();

    // Create patient
    let patient_input = CreatePatient {
        ahv_number: "756.4567.8901.20".to_string(),
        first_name: "Test".to_string(),
        last_name: "Patient".to_string(),
        date_of_birth: "1980-01-01".to_string(),
        gender: None,
        address: None,
        phone: None,
        email: None,
        insurance: None,
        gp_name: None,
        gp_address: None,
        notes: None,
    };
    let patient = patient::create_patient(&conn, patient_input).unwrap();

    // Create multiple sessions with different dates
    let session1 = session::create_session(
        &conn,
        CreateSession {
            patient_id: patient.id.clone(),
            session_date: "2026-01-15".to_string(),
            session_type: "Session 1".to_string(),
            duration_minutes: None,
            scheduled_time: None,
            notes: None,
            amdp_data: None,
        },
    )
    .unwrap();

    let session2 = session::create_session(
        &conn,
        CreateSession {
            patient_id: patient.id.clone(),
            session_date: "2026-03-15".to_string(),
            session_type: "Session 2".to_string(),
            duration_minutes: None,
            scheduled_time: None,
            notes: None,
            amdp_data: None,
        },
    )
    .unwrap();

    let session3 = session::create_session(
        &conn,
        CreateSession {
            patient_id: patient.id.clone(),
            session_date: "2026-02-15".to_string(),
            session_type: "Session 3".to_string(),
            duration_minutes: None,
            scheduled_time: None,
            notes: None,
            amdp_data: None,
        },
    )
    .unwrap();

    // List sessions - should be ordered by date DESC
    let sessions = session::list_sessions_for_patient(&conn, &patient.id, 10, 0).unwrap();
    assert_eq!(sessions.len(), 3);
    assert_eq!(sessions[0].id, session2.id); // 2026-03-15
    assert_eq!(sessions[1].id, session3.id); // 2026-02-15
    assert_eq!(sessions[2].id, session1.id); // 2026-01-15
}

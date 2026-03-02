#![allow(dead_code)]

mod ahv;
pub mod audit;
mod commands;
mod constants;
mod crypto;
mod database;
mod error;
mod filesystem; // PKG-3: Encrypted Filesystem
mod keychain;
mod llm;
mod models;
mod recovery;
mod search;
mod spotlight; // PKG-3: macOS Spotlight exclusion
mod state;

#[cfg(test)]
mod integration_tests;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let data_dir = dirs::home_dir().unwrap_or_default().join("DokAssist");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new(data_dir))
        .invoke_handler(tauri::generate_handler![
            commands::audit::query_audit_log,
            commands::auth::check_auth,
            commands::auth::initialize_app,
            commands::auth::unlock_app,
            commands::auth::recover_app,
            commands::auth::lock_app,
            commands::patients::create_patient,
            commands::patients::get_patient,
            commands::patients::list_patients,
            commands::patients::update_patient,
            commands::patients::delete_patient,
            commands::files::upload_file,
            commands::files::download_file,
            commands::files::list_files,
            commands::files::delete_file,
            commands::sessions::create_session,
            commands::sessions::get_session,
            commands::sessions::list_sessions_for_patient,
            commands::sessions::update_session,
            commands::sessions::delete_session,
            commands::diagnoses::create_diagnosis,
            commands::diagnoses::get_diagnosis,
            commands::diagnoses::list_diagnoses_for_patient,
            commands::diagnoses::update_diagnosis,
            commands::diagnoses::delete_diagnosis,
            commands::medications::create_medication,
            commands::medications::get_medication,
            commands::medications::list_medications_for_patient,
            commands::medications::update_medication,
            commands::medications::delete_medication,
            commands::search::search_patients,
            commands::search::global_search,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::llm::get_engine_status,
            commands::llm::get_recommended_model,
            commands::llm::get_default_system_prompt,
            commands::llm::download_model,
            commands::llm::load_model,
            commands::llm::extract_file_metadata,
            commands::llm::generate_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

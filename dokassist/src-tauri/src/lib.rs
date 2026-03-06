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
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let data_dir = dirs::home_dir().unwrap_or_default().join("DokAssist");

    // Ensure the data directory exists before state is initialised.
    // SQLite's OPEN_CREATE flag only creates the *file*, not parent directories.
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        log::error!(
            "Failed to create data directory {}: {}",
            data_dir.display(),
            e
        );
    }

    let app_state = AppState::new(data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::audit::query_audit_log,
            commands::auth::check_auth,
            commands::auth::initialize_app,
            commands::auth::unlock_app,
            commands::auth::recover_app,
            commands::auth::lock_app,
            commands::auth::reset_app,
            commands::patients::create_patient,
            commands::patients::get_patient,
            commands::patients::list_patients,
            commands::patients::update_patient,
            commands::patients::delete_patient,
            commands::files::upload_file,
            commands::files::download_file,
            commands::files::list_files,
            commands::files::delete_file,
            commands::files::process_file,
            commands::sessions::create_session,
            commands::sessions::get_session,
            commands::sessions::list_all_sessions,
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
            commands::llm::improve_text,
            commands::llm::get_embed_status,
            commands::llm::initialize_embed_engine,
            commands::reports::create_report,
            commands::reports::get_report,
            commands::reports::list_reports,
            commands::reports::update_report,
            commands::reports::delete_report,
            commands::reports::export_report_to_pdf,
            commands::reports::export_report_to_docx,
            commands::updater::check_for_updates,
            commands::updater::install_update,
            commands::updater::get_app_version,
            commands::export::export_all_patient_data,
            commands::chat::run_agent_turn,
            commands::chat::create_chat_session,
            commands::chat::get_or_create_patient_chat_session,
            commands::chat::list_chat_sessions,
            commands::chat::delete_chat_session,
            commands::chat::get_chat_messages,
            commands::chat::rename_chat_session,
            commands::literature::upload_literature,
            commands::literature::get_literature_by_id,
            commands::literature::list_all_literature,
            commands::literature::update_literature_metadata,
            commands::literature::delete_literature_document,
            commands::literature::download_literature,
            commands::literature::process_literature,
            commands::literature::search_literature,
            commands::literature::get_literature_document_chunks,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Gracefully unload LLM model before closing
                if let Some(state) = window.try_state::<AppState>() {
                    log::info!("Window close requested - cleaning up LLM resources");
                    state.clear_llm();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

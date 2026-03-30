use crate::error::AppError;
use crate::models::settings::PracticeSettings;
use crate::state::AppState;
use tauri::State;

/// Get practice settings from the database
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<PracticeSettings, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let mut stmt = conn.prepare(
        "SELECT practice_name, practice_address, practice_phone, practice_email,
                therapist_name, zsr_number, canton, clinical_specialty,
                language_preference, onboarding_completed
         FROM practice_settings WHERE id = 1",
    )?;

    let settings = stmt.query_row([], |row: &rusqlite::Row<'_>| {
        Ok(PracticeSettings {
            practice_name: row.get(0)?,
            practice_address: row.get(1)?,
            practice_phone: row.get(2)?,
            practice_email: row.get(3)?,
            therapist_name: row.get(4)?,
            zsr_number: row.get(5)?,
            canton: row.get(6)?,
            clinical_specialty: row.get(7)?,
            language_preference: row.get(8)?,
            onboarding_completed: row.get::<_, i32>(9)? != 0,
        })
    })?;

    Ok(settings)
}

/// Update practice settings in the database
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: PracticeSettings,
) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    conn.execute(
        "UPDATE practice_settings
         SET practice_name = ?1,
             practice_address = ?2,
             practice_phone = ?3,
             practice_email = ?4,
             therapist_name = ?5,
             zsr_number = ?6,
             canton = ?7,
             clinical_specialty = ?8,
             language_preference = ?9,
             onboarding_completed = ?10
         WHERE id = 1",
        (
            &settings.practice_name,
            &settings.practice_address,
            &settings.practice_phone,
            &settings.practice_email,
            &settings.therapist_name,
            &settings.zsr_number,
            &settings.canton,
            &settings.clinical_specialty,
            &settings.language_preference,
            if settings.onboarding_completed { 1 } else { 0 },
        ),
    )?;

    Ok(())
}

/// Mark onboarding as completed
#[tauri::command]
pub async fn complete_onboarding(state: State<'_, AppState>) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    conn.execute(
        "UPDATE practice_settings SET onboarding_completed = 1 WHERE id = 1",
        [],
    )?;

    Ok(())
}


}


}


}


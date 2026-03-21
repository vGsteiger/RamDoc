use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::treatment_plan::{
    self, CreateTreatmentGoal, CreateTreatmentIntervention, CreateTreatmentPlan, TreatmentGoal,
    TreatmentIntervention, TreatmentPlan, UpdateTreatmentGoal, UpdateTreatmentIntervention,
    UpdateTreatmentPlan,
};
use crate::search;
use crate::state::AppState;
use tauri::State;

// ========== Treatment Plan Commands ==========

#[tauri::command]
pub async fn create_treatment_plan(
    state: State<'_, AppState>,
    input: CreateTreatmentPlan,
) -> Result<TreatmentPlan, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let plan = treatment_plan::create_treatment_plan(&tx, input)?;

    audit::log(
        &tx,
        AuditAction::Create,
        "treatment_plan",
        Some(&plan.id),
        None,
    )?;

    tx.commit()?;

    search::index_treatment_plan_from_model(&conn, &plan)?;

    Ok(plan)
}

#[tauri::command]
pub async fn get_treatment_plan(
    state: State<'_, AppState>,
    id: String,
) -> Result<TreatmentPlan, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let plan = treatment_plan::get_treatment_plan(&conn, &id)?;

    audit::log(&conn, AuditAction::View, "treatment_plan", Some(&id), None)?;

    Ok(plan)
}

#[tauri::command]
pub async fn list_treatment_plans_for_patient(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<TreatmentPlan>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let plans =
        treatment_plan::list_treatment_plans_for_patient(&conn, &patient_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "treatment_plan",
        None,
        Some(&format!(
            "list: {} treatment plans for patient {}",
            plans.len(),
            patient_id
        )),
    )?;

    Ok(plans)
}

#[tauri::command]
pub async fn update_treatment_plan(
    state: State<'_, AppState>,
    id: String,
    input: UpdateTreatmentPlan,
) -> Result<TreatmentPlan, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let plan = treatment_plan::update_treatment_plan(&tx, &id, input)?;

    audit::log(&tx, AuditAction::Update, "treatment_plan", Some(&id), None)?;

    tx.commit()?;

    search::index_treatment_plan_from_model(&conn, &plan)?;

    Ok(plan)
}

#[tauri::command]
pub async fn delete_treatment_plan(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    // Get all goals and interventions before deleting the plan
    let goals = treatment_plan::list_treatment_goals_for_plan(&tx, &id, 1000, 0)?;
    let interventions = treatment_plan::list_treatment_interventions_for_plan(&tx, &id, 1000, 0)?;

    // Remove all search index entries (plan, goals, interventions)
    search::remove_from_index(&tx, "treatment_plan", &id)?;
    for goal in goals {
        search::remove_from_index(&tx, "treatment_goal", &goal.id)?;
    }
    for intervention in interventions {
        search::remove_from_index(&tx, "treatment_intervention", &intervention.id)?;
    }

    treatment_plan::delete_treatment_plan(&tx, &id)?;

    audit::log(&tx, AuditAction::Delete, "treatment_plan", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}

// ========== Treatment Goal Commands ==========

#[tauri::command]
pub async fn create_treatment_goal(
    state: State<'_, AppState>,
    input: CreateTreatmentGoal,
) -> Result<TreatmentGoal, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let goal = treatment_plan::create_treatment_goal(&tx, input)?;

    audit::log(
        &tx,
        AuditAction::Create,
        "treatment_goal",
        Some(&goal.id),
        None,
    )?;

    tx.commit()?;

    search::index_treatment_goal_from_model(&conn, &goal)?;

    Ok(goal)
}

#[tauri::command]
pub async fn get_treatment_goal(
    state: State<'_, AppState>,
    id: String,
) -> Result<TreatmentGoal, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let goal = treatment_plan::get_treatment_goal(&conn, &id)?;

    audit::log(&conn, AuditAction::View, "treatment_goal", Some(&id), None)?;

    Ok(goal)
}

#[tauri::command]
pub async fn list_treatment_goals_for_plan(
    state: State<'_, AppState>,
    plan_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<TreatmentGoal>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let goals = treatment_plan::list_treatment_goals_for_plan(&conn, &plan_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "treatment_goal",
        None,
        Some(&format!(
            "list: {} treatment goals for plan {}",
            goals.len(),
            plan_id
        )),
    )?;

    Ok(goals)
}

#[tauri::command]
pub async fn update_treatment_goal(
    state: State<'_, AppState>,
    id: String,
    input: UpdateTreatmentGoal,
) -> Result<TreatmentGoal, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let goal = treatment_plan::update_treatment_goal(&tx, &id, input)?;

    audit::log(&tx, AuditAction::Update, "treatment_goal", Some(&id), None)?;

    tx.commit()?;

    search::index_treatment_goal_from_model(&conn, &goal)?;

    Ok(goal)
}

#[tauri::command]
pub async fn delete_treatment_goal(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    search::remove_from_index(&tx, "treatment_goal", &id)?;

    treatment_plan::delete_treatment_goal(&tx, &id)?;

    audit::log(&tx, AuditAction::Delete, "treatment_goal", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}

// ========== Treatment Intervention Commands ==========

#[tauri::command]
pub async fn create_treatment_intervention(
    state: State<'_, AppState>,
    input: CreateTreatmentIntervention,
) -> Result<TreatmentIntervention, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let intervention = treatment_plan::create_treatment_intervention(&tx, input)?;

    audit::log(
        &tx,
        AuditAction::Create,
        "treatment_intervention",
        Some(&intervention.id),
        None,
    )?;

    tx.commit()?;

    search::index_treatment_intervention_from_model(&conn, &intervention)?;

    Ok(intervention)
}

#[tauri::command]
pub async fn get_treatment_intervention(
    state: State<'_, AppState>,
    id: String,
) -> Result<TreatmentIntervention, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let intervention = treatment_plan::get_treatment_intervention(&conn, &id)?;

    audit::log(
        &conn,
        AuditAction::View,
        "treatment_intervention",
        Some(&id),
        None,
    )?;

    Ok(intervention)
}

#[tauri::command]
pub async fn list_treatment_interventions_for_plan(
    state: State<'_, AppState>,
    plan_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<TreatmentIntervention>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let interventions =
        treatment_plan::list_treatment_interventions_for_plan(&conn, &plan_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "treatment_intervention",
        None,
        Some(&format!(
            "list: {} treatment interventions for plan {}",
            interventions.len(),
            plan_id
        )),
    )?;

    Ok(interventions)
}

#[tauri::command]
pub async fn update_treatment_intervention(
    state: State<'_, AppState>,
    id: String,
    input: UpdateTreatmentIntervention,
) -> Result<TreatmentIntervention, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let intervention = treatment_plan::update_treatment_intervention(&tx, &id, input)?;

    audit::log(
        &tx,
        AuditAction::Update,
        "treatment_intervention",
        Some(&id),
        None,
    )?;

    tx.commit()?;

    search::index_treatment_intervention_from_model(&conn, &intervention)?;

    Ok(intervention)
}

#[tauri::command]
pub async fn delete_treatment_intervention(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    search::remove_from_index(&tx, "treatment_intervention", &id)?;

    treatment_plan::delete_treatment_intervention(&tx, &id)?;

    audit::log(
        &tx,
        AuditAction::Delete,
        "treatment_intervention",
        Some(&id),
        None,
    )?;

    tx.commit()?;

    Ok(())
}

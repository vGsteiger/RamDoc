use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ========== Treatment Plan ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatmentPlan {
    pub id: String,
    pub patient_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTreatmentPlan {
    pub patient_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTreatmentPlan {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
}

fn row_to_treatment_plan(row: &Row) -> Result<TreatmentPlan, rusqlite::Error> {
    Ok(TreatmentPlan {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        start_date: row.get(4)?,
        end_date: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn create_treatment_plan(
    conn: &Connection,
    input: CreateTreatmentPlan,
) -> Result<TreatmentPlan, AppError> {
    let id = Uuid::now_v7().to_string();
    let status = input.status.unwrap_or_else(|| "active".to_string());

    conn.execute(
        "INSERT INTO treatment_plans (id, patient_id, title, description, start_date, end_date, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.title,
            input.description,
            input.start_date,
            input.end_date,
            status,
        ],
    )?;

    get_treatment_plan(conn, &id)
}

pub fn get_treatment_plan(conn: &Connection, id: &str) -> Result<TreatmentPlan, AppError> {
    let plan = conn
        .query_row(
            "SELECT id, patient_id, title, description, start_date, end_date, status,
                    created_at, updated_at
             FROM treatment_plans WHERE id = ?",
            params![id],
            row_to_treatment_plan,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Treatment plan not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(plan)
}

pub fn update_treatment_plan(
    conn: &Connection,
    id: &str,
    input: UpdateTreatmentPlan,
) -> Result<TreatmentPlan, AppError> {
    get_treatment_plan(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(title) = input.title {
        updates.push("title = ?");
        values.push(Box::new(title));
    }
    if let Some(description) = input.description {
        updates.push("description = ?");
        values.push(Box::new(description));
    }
    if let Some(start_date) = input.start_date {
        updates.push("start_date = ?");
        values.push(Box::new(start_date));
    }
    if let Some(end_date) = input.end_date {
        updates.push("end_date = ?");
        values.push(Box::new(end_date));
    }
    if let Some(status) = input.status {
        updates.push("status = ?");
        values.push(Box::new(status));
    }

    if updates.is_empty() {
        return get_treatment_plan(conn, id);
    }

    let query = format!(
        "UPDATE treatment_plans SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_treatment_plan(conn, id)
}

pub fn delete_treatment_plan(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM treatment_plans WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Treatment plan not found: {}",
            id
        )));
    }

    Ok(())
}

pub fn list_treatment_plans_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<TreatmentPlan>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, title, description, start_date, end_date, status,
                created_at, updated_at
         FROM treatment_plans
         WHERE patient_id = ?
         ORDER BY start_date DESC
         LIMIT ? OFFSET ?",
    )?;

    let plans = stmt
        .query_map(params![patient_id, limit, offset], row_to_treatment_plan)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(plans)
}

// ========== Treatment Goal ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatmentGoal {
    pub id: String,
    pub treatment_plan_id: String,
    pub description: String,
    pub target_date: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTreatmentGoal {
    pub treatment_plan_id: String,
    pub description: String,
    pub target_date: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTreatmentGoal {
    pub description: Option<String>,
    pub target_date: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}

fn row_to_treatment_goal(row: &Row) -> Result<TreatmentGoal, rusqlite::Error> {
    Ok(TreatmentGoal {
        id: row.get(0)?,
        treatment_plan_id: row.get(1)?,
        description: row.get(2)?,
        target_date: row.get(3)?,
        status: row.get(4)?,
        sort_order: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub fn create_treatment_goal(
    conn: &Connection,
    input: CreateTreatmentGoal,
) -> Result<TreatmentGoal, AppError> {
    let id = Uuid::now_v7().to_string();
    let status = input.status.unwrap_or_else(|| "in_progress".to_string());
    let sort_order = input.sort_order.unwrap_or(0);

    conn.execute(
        "INSERT INTO treatment_goals (id, treatment_plan_id, description, target_date, status, sort_order)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.treatment_plan_id,
            input.description,
            input.target_date,
            status,
            sort_order,
        ],
    )?;

    get_treatment_goal(conn, &id)
}

pub fn get_treatment_goal(conn: &Connection, id: &str) -> Result<TreatmentGoal, AppError> {
    let goal = conn
        .query_row(
            "SELECT id, treatment_plan_id, description, target_date, status, sort_order,
                    created_at, updated_at
             FROM treatment_goals WHERE id = ?",
            params![id],
            row_to_treatment_goal,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Treatment goal not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(goal)
}

pub fn update_treatment_goal(
    conn: &Connection,
    id: &str,
    input: UpdateTreatmentGoal,
) -> Result<TreatmentGoal, AppError> {
    get_treatment_goal(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(description) = input.description {
        updates.push("description = ?");
        values.push(Box::new(description));
    }
    if let Some(target_date) = input.target_date {
        updates.push("target_date = ?");
        values.push(Box::new(target_date));
    }
    if let Some(status) = input.status {
        updates.push("status = ?");
        values.push(Box::new(status));
    }
    if let Some(sort_order) = input.sort_order {
        updates.push("sort_order = ?");
        values.push(Box::new(sort_order));
    }

    if updates.is_empty() {
        return get_treatment_goal(conn, id);
    }

    let query = format!(
        "UPDATE treatment_goals SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_treatment_goal(conn, id)
}

pub fn delete_treatment_goal(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM treatment_goals WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Treatment goal not found: {}",
            id
        )));
    }

    Ok(())
}

pub fn list_treatment_goals_for_plan(
    conn: &Connection,
    plan_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<TreatmentGoal>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, treatment_plan_id, description, target_date, status, sort_order,
                created_at, updated_at
         FROM treatment_goals
         WHERE treatment_plan_id = ?
         ORDER BY sort_order ASC, created_at ASC
         LIMIT ? OFFSET ?",
    )?;

    let goals = stmt
        .query_map(params![plan_id, limit, offset], row_to_treatment_goal)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(goals)
}

// ========== Treatment Intervention ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatmentIntervention {
    pub id: String,
    pub treatment_plan_id: String,
    pub r#type: String,
    pub description: String,
    pub frequency: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTreatmentIntervention {
    pub treatment_plan_id: String,
    pub r#type: String,
    pub description: String,
    pub frequency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTreatmentIntervention {
    pub r#type: Option<String>,
    pub description: Option<String>,
    pub frequency: Option<String>,
}

fn row_to_treatment_intervention(row: &Row) -> Result<TreatmentIntervention, rusqlite::Error> {
    Ok(TreatmentIntervention {
        id: row.get(0)?,
        treatment_plan_id: row.get(1)?,
        r#type: row.get(2)?,
        description: row.get(3)?,
        frequency: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub fn create_treatment_intervention(
    conn: &Connection,
    input: CreateTreatmentIntervention,
) -> Result<TreatmentIntervention, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO treatment_interventions (id, treatment_plan_id, type, description, frequency)
         VALUES (?, ?, ?, ?, ?)",
        params![
            id,
            input.treatment_plan_id,
            input.r#type,
            input.description,
            input.frequency,
        ],
    )?;

    get_treatment_intervention(conn, &id)
}

pub fn get_treatment_intervention(
    conn: &Connection,
    id: &str,
) -> Result<TreatmentIntervention, AppError> {
    let intervention = conn
        .query_row(
            "SELECT id, treatment_plan_id, type, description, frequency,
                    created_at, updated_at
             FROM treatment_interventions WHERE id = ?",
            params![id],
            row_to_treatment_intervention,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Treatment intervention not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(intervention)
}

pub fn update_treatment_intervention(
    conn: &Connection,
    id: &str,
    input: UpdateTreatmentIntervention,
) -> Result<TreatmentIntervention, AppError> {
    get_treatment_intervention(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(r#type) = input.r#type {
        updates.push("type = ?");
        values.push(Box::new(r#type));
    }
    if let Some(description) = input.description {
        updates.push("description = ?");
        values.push(Box::new(description));
    }
    if let Some(frequency) = input.frequency {
        updates.push("frequency = ?");
        values.push(Box::new(frequency));
    }

    if updates.is_empty() {
        return get_treatment_intervention(conn, id);
    }

    let query = format!(
        "UPDATE treatment_interventions SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_treatment_intervention(conn, id)
}

pub fn delete_treatment_intervention(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute(
        "DELETE FROM treatment_interventions WHERE id = ?",
        params![id],
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Treatment intervention not found: {}",
            id
        )));
    }

    Ok(())
}

pub fn list_treatment_interventions_for_plan(
    conn: &Connection,
    plan_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<TreatmentIntervention>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, treatment_plan_id, type, description, frequency,
                created_at, updated_at
         FROM treatment_interventions
         WHERE treatment_plan_id = ?
         ORDER BY created_at ASC
         LIMIT ? OFFSET ?",
    )?;

    let interventions = stmt
        .query_map(
            params![plan_id, limit, offset],
            row_to_treatment_intervention,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(interventions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use tempfile::tempdir;

    fn open_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let dir = tempdir().unwrap();
        let key = crate::crypto::generate_key();
        let pool = init_db(&dir.path().join("test.db"), &key).unwrap();
        (dir, pool)
    }

    fn insert_patient(conn: &Connection) {
        conn.execute(
            "INSERT INTO patients (id, first_name, last_name, date_of_birth, ahv_number)
             VALUES ('p1', 'Anna', 'Test', '1985-01-01', '756.1234.5678.97')",
            [],
        )
        .unwrap();
    }

    fn make_plan(conn: &Connection) -> TreatmentPlan {
        create_treatment_plan(
            conn,
            CreateTreatmentPlan {
                patient_id: "p1".into(),
                title: "CBT Plan".into(),
                description: None,
                start_date: "2026-01-01".into(),
                end_date: None,
                status: None,
            },
        )
        .unwrap()
    }

    // ---- TreatmentPlan ----

    #[test]
    fn test_create_and_get_treatment_plan() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let p = make_plan(&conn);
        assert_eq!(p.title, "CBT Plan");
        assert_eq!(p.patient_id, "p1");
        let p2 = get_treatment_plan(&conn, &p.id).unwrap();
        assert_eq!(p.id, p2.id);
    }

    #[test]
    fn test_create_treatment_plan_default_status() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let p = make_plan(&conn);
        assert_eq!(p.status, "active");
    }

    #[test]
    fn test_update_treatment_plan() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let p = make_plan(&conn);
        let updated = update_treatment_plan(
            &conn,
            &p.id,
            UpdateTreatmentPlan {
                title: Some("DBT Plan".into()),
                description: None,
                start_date: None,
                end_date: None,
                status: Some("completed".into()),
            },
        )
        .unwrap();
        assert_eq!(updated.title, "DBT Plan");
        assert_eq!(updated.status, "completed");
    }

    #[test]
    fn test_delete_treatment_plan() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let p = make_plan(&conn);
        delete_treatment_plan(&conn, &p.id).unwrap();
        assert!(matches!(
            get_treatment_plan(&conn, &p.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_treatment_plans_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        for (title, date) in [
            ("Plan A", "2026-01-01"),
            ("Plan B", "2026-02-01"),
            ("Plan C", "2026-03-01"),
        ] {
            create_treatment_plan(
                &conn,
                CreateTreatmentPlan {
                    patient_id: "p1".into(),
                    title: title.into(),
                    description: None,
                    start_date: date.into(),
                    end_date: None,
                    status: None,
                },
            )
            .unwrap();
        }
        let list = list_treatment_plans_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].start_date, "2026-03-01");
    }

    // ---- TreatmentGoal ----

    #[test]
    fn test_create_and_get_treatment_goal() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let g = create_treatment_goal(
            &conn,
            CreateTreatmentGoal {
                treatment_plan_id: plan.id.clone(),
                description: "Reduce anxiety".into(),
                target_date: None,
                status: None,
                sort_order: None,
            },
        )
        .unwrap();
        assert_eq!(g.status, "in_progress");
        assert_eq!(g.sort_order, 0);
        assert_eq!(g.description, "Reduce anxiety");
        let g2 = get_treatment_goal(&conn, &g.id).unwrap();
        assert_eq!(g.id, g2.id);
    }

    #[test]
    fn test_update_treatment_goal() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let g = create_treatment_goal(
            &conn,
            CreateTreatmentGoal {
                treatment_plan_id: plan.id.clone(),
                description: "Improve sleep".into(),
                target_date: None,
                status: None,
                sort_order: Some(1),
            },
        )
        .unwrap();
        let updated = update_treatment_goal(
            &conn,
            &g.id,
            UpdateTreatmentGoal {
                description: None,
                target_date: None,
                status: Some("achieved".into()),
                sort_order: Some(5),
            },
        )
        .unwrap();
        assert_eq!(updated.status, "achieved");
        assert_eq!(updated.sort_order, 5);
        assert_eq!(updated.description, "Improve sleep");
    }

    #[test]
    fn test_delete_treatment_goal() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let g = create_treatment_goal(
            &conn,
            CreateTreatmentGoal {
                treatment_plan_id: plan.id.clone(),
                description: "Goal to delete".into(),
                target_date: None,
                status: None,
                sort_order: None,
            },
        )
        .unwrap();
        delete_treatment_goal(&conn, &g.id).unwrap();
        assert!(matches!(
            get_treatment_goal(&conn, &g.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_treatment_goals_for_plan() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        for (desc, order) in [("Goal A", 2), ("Goal B", 0), ("Goal C", 1)] {
            create_treatment_goal(
                &conn,
                CreateTreatmentGoal {
                    treatment_plan_id: plan.id.clone(),
                    description: desc.into(),
                    target_date: None,
                    status: None,
                    sort_order: Some(order),
                },
            )
            .unwrap();
        }
        let list = list_treatment_goals_for_plan(&conn, &plan.id, 10, 0).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].sort_order, 0);
        assert_eq!(list[2].sort_order, 2);
    }

    // ---- TreatmentIntervention ----

    #[test]
    fn test_create_and_get_treatment_intervention() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let i = create_treatment_intervention(
            &conn,
            CreateTreatmentIntervention {
                treatment_plan_id: plan.id.clone(),
                r#type: "psychotherapy".into(),
                description: "Exposure therapy".into(),
                frequency: Some("weekly".into()),
            },
        )
        .unwrap();
        assert_eq!(i.r#type, "psychotherapy");
        assert_eq!(i.description, "Exposure therapy");
        assert_eq!(i.frequency, Some("weekly".into()));
        let i2 = get_treatment_intervention(&conn, &i.id).unwrap();
        assert_eq!(i.id, i2.id);
    }

    #[test]
    fn test_update_treatment_intervention() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let i = create_treatment_intervention(
            &conn,
            CreateTreatmentIntervention {
                treatment_plan_id: plan.id.clone(),
                r#type: "psychotherapy".into(),
                description: "Thought records".into(),
                frequency: None,
            },
        )
        .unwrap();
        let updated = update_treatment_intervention(
            &conn,
            &i.id,
            UpdateTreatmentIntervention {
                r#type: None,
                description: Some("Thought diary".into()),
                frequency: Some("daily".into()),
            },
        )
        .unwrap();
        assert_eq!(updated.description, "Thought diary");
        assert_eq!(updated.r#type, "psychotherapy");
        assert_eq!(updated.frequency, Some("daily".into()));
    }

    #[test]
    fn test_delete_treatment_intervention() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let i = create_treatment_intervention(
            &conn,
            CreateTreatmentIntervention {
                treatment_plan_id: plan.id.clone(),
                r#type: "other".into(),
                description: "Breathing exercises".into(),
                frequency: None,
            },
        )
        .unwrap();
        delete_treatment_intervention(&conn, &i.id).unwrap();
        assert!(matches!(
            get_treatment_intervention(&conn, &i.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_treatment_interventions_for_plan() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        for desc in ["Intervention A", "Intervention B", "Intervention C"] {
            create_treatment_intervention(
                &conn,
                CreateTreatmentIntervention {
                    treatment_plan_id: plan.id.clone(),
                    r#type: "referral".into(),
                    description: desc.into(),
                    frequency: None,
                },
            )
            .unwrap();
        }
        let list = list_treatment_interventions_for_plan(&conn, &plan.id, 10, 0).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_update_intervention_no_fields() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let plan = make_plan(&conn);
        let i = create_treatment_intervention(
            &conn,
            CreateTreatmentIntervention {
                treatment_plan_id: plan.id.clone(),
                r#type: "medication".into(),
                description: "Depression education".into(),
                frequency: Some("once".into()),
            },
        )
        .unwrap();
        let unchanged = update_treatment_intervention(
            &conn,
            &i.id,
            UpdateTreatmentIntervention {
                r#type: None,
                description: None,
                frequency: None,
            },
        )
        .unwrap();
        assert_eq!(unchanged.description, "Depression education");
        assert_eq!(unchanged.r#type, "medication");
    }
}

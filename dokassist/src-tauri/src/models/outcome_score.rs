use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeScore {
    pub id: String,
    pub session_id: String,
    pub scale_type: String,
    pub score: i32,
    pub interpretation: Option<String>,
    pub subscores: Option<String>,
    pub administered_at: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOutcomeScore {
    pub session_id: String,
    pub scale_type: String,
    pub score: i32,
    pub subscores: Option<String>,
    pub administered_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOutcomeScore {
    pub scale_type: Option<String>,
    pub score: Option<i32>,
    pub subscores: Option<String>,
    pub administered_at: Option<String>,
    pub notes: Option<String>,
}

/// Validate and normalize scale type
pub fn validate_scale_type(scale_type: &str) -> Result<String, AppError> {
    match scale_type
        .to_uppercase()
        .replace("-", "")
        .replace(" ", "")
        .as_str()
    {
        "PHQ9" | "PHQ-9" => Ok("PHQ-9".to_string()),
        "GAD7" | "GAD-7" => Ok("GAD-7".to_string()),
        "BDIII" | "BDI-II" | "BDI2" | "BDII" => Ok("BDI-II".to_string()),
        _ => Err(AppError::Validation(format!(
            "Unknown questionnaire type: {}. Must be PHQ-9, GAD-7, or BDI-II",
            scale_type
        ))),
    }
}

/// Calculate interpretation for PHQ-9 score (0-27)
pub fn interpret_phq9(score: i32) -> Result<String, AppError> {
    match score {
        0..=4 => Ok("Minimal".to_string()),
        5..=9 => Ok("Mild".to_string()),
        10..=14 => Ok("Moderate".to_string()),
        15..=19 => Ok("Moderately Severe".to_string()),
        20..=27 => Ok("Severe".to_string()),
        _ => Err(AppError::Validation(format!(
            "PHQ-9 score must be 0-27, got {}",
            score
        ))),
    }
}

/// Calculate interpretation for GAD-7 score (0-21)
pub fn interpret_gad7(score: i32) -> Result<String, AppError> {
    match score {
        0..=4 => Ok("Minimal".to_string()),
        5..=9 => Ok("Mild".to_string()),
        10..=14 => Ok("Moderate".to_string()),
        15..=21 => Ok("Severe".to_string()),
        _ => Err(AppError::Validation(format!(
            "GAD-7 score must be 0-21, got {}",
            score
        ))),
    }
}

/// Calculate interpretation for BDI-II score (0-63)
pub fn interpret_bdi_ii(score: i32) -> Result<String, AppError> {
    match score {
        0..=13 => Ok("Minimal".to_string()),
        14..=19 => Ok("Mild".to_string()),
        20..=28 => Ok("Moderate".to_string()),
        29..=63 => Ok("Severe".to_string()),
        _ => Err(AppError::Validation(format!(
            "BDI-II score must be 0-63, got {}",
            score
        ))),
    }
}

/// Calculate interpretation based on scale type and score
pub fn calculate_interpretation(scale_type: &str, score: i32) -> Result<String, AppError> {
    match scale_type {
        "PHQ-9" => interpret_phq9(score),
        "GAD-7" => interpret_gad7(score),
        "BDI-II" => interpret_bdi_ii(score),
        _ => Err(AppError::Validation(format!(
            "Unknown scale type: {}",
            scale_type
        ))),
    }
}

fn row_to_outcome_score(row: &Row) -> Result<OutcomeScore, rusqlite::Error> {
    Ok(OutcomeScore {
        id: row.get(0)?,
        session_id: row.get(1)?,
        scale_type: row.get(2)?,
        score: row.get(3)?,
        interpretation: row.get(4)?,
        subscores: row.get(5)?,
        administered_at: row.get(6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn create_outcome_score(
    conn: &Connection,
    input: CreateOutcomeScore,
) -> Result<OutcomeScore, AppError> {
    // Validate scale type
    let scale_type = validate_scale_type(&input.scale_type)?;

    // Calculate interpretation
    let interpretation = calculate_interpretation(&scale_type, input.score)?;

    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO outcome_scores
         (id, session_id, scale_type, score, interpretation, subscores, administered_at, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.session_id,
            scale_type,
            input.score,
            interpretation,
            input.subscores,
            input.administered_at,
            input.notes,
        ],
    )?;

    get_outcome_score(conn, &id)
}

pub fn get_outcome_score(conn: &Connection, id: &str) -> Result<OutcomeScore, AppError> {
    let score = conn
        .query_row(
            "SELECT id, session_id, scale_type, score, interpretation, subscores,
                    administered_at, notes, created_at, updated_at
             FROM outcome_scores
             WHERE id = ?",
            params![id],
            row_to_outcome_score,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Outcome score not found: {}", id))
            }
            other => AppError::from(other),
        })?;
    Ok(score)
}

pub fn list_scores_for_session(
    conn: &Connection,
    session_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<OutcomeScore>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, scale_type, score, interpretation, subscores,
                administered_at, notes, created_at, updated_at
         FROM outcome_scores
         WHERE session_id = ?
         ORDER BY administered_at DESC
         LIMIT ? OFFSET ?",
    )?;

    let scores = stmt
        .query_map(params![session_id, limit, offset], row_to_outcome_score)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scores)
}

pub fn list_scores_by_scale(
    conn: &Connection,
    scale_type: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<OutcomeScore>, AppError> {
    let validated_scale = validate_scale_type(scale_type)?;

    let mut stmt = conn.prepare(
        "SELECT id, session_id, scale_type, score, interpretation, subscores,
                administered_at, notes, created_at, updated_at
         FROM outcome_scores
         WHERE scale_type = ?
         ORDER BY administered_at DESC
         LIMIT ? OFFSET ?",
    )?;

    let scores = stmt
        .query_map(
            params![validated_scale, limit, offset],
            row_to_outcome_score,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scores)
}

pub fn update_outcome_score(
    conn: &Connection,
    id: &str,
    input: UpdateOutcomeScore,
) -> Result<OutcomeScore, AppError> {
    // Verify the score exists
    get_outcome_score(conn, id)?;

    // Destructure upfront to avoid use-after-move
    let scale_type_opt = input.scale_type;
    let score_opt = input.score;
    let subscores_opt = input.subscores;
    let administered_at_opt = input.administered_at;
    let notes_opt = input.notes;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref scale_type) = scale_type_opt {
        let validated_scale = validate_scale_type(scale_type)?;
        updates.push("scale_type = ?");
        values.push(Box::new(validated_scale));
    }

    if let Some(score) = score_opt {
        updates.push("score = ?");
        values.push(Box::new(score));
    }

    if let Some(subscores) = subscores_opt {
        updates.push("subscores = ?");
        values.push(Box::new(subscores));
    }

    if let Some(administered_at) = administered_at_opt {
        updates.push("administered_at = ?");
        values.push(Box::new(administered_at));
    }

    if let Some(notes) = notes_opt {
        updates.push("notes = ?");
        values.push(Box::new(notes));
    }

    if updates.is_empty() {
        return get_outcome_score(conn, id);
    }

    // Recalculate interpretation if score or scale_type changed
    if score_opt.is_some() || scale_type_opt.is_some() {
        let current = get_outcome_score(conn, id)?;
        let new_scale = scale_type_opt
            .as_ref()
            .map(|s| validate_scale_type(s))
            .transpose()?
            .unwrap_or(current.scale_type);
        let new_score = score_opt.unwrap_or(current.score);
        let new_interpretation = calculate_interpretation(&new_scale, new_score)?;

        updates.push("interpretation = ?");
        values.push(Box::new(new_interpretation));
    }

    let query = format!(
        "UPDATE outcome_scores SET {} WHERE id = ?",
        updates.join(", ")
    );
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_outcome_score(conn, id)
}

pub fn delete_outcome_score(conn: &Connection, id: &str) -> Result<(), AppError> {
    // Verify the score exists
    get_outcome_score(conn, id)?;

    let rows_affected = conn.execute("DELETE FROM outcome_scores WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Outcome score not found: {}",
            id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_scale_type() {
        assert_eq!(validate_scale_type("PHQ-9").unwrap(), "PHQ-9");
        assert_eq!(validate_scale_type("phq9").unwrap(), "PHQ-9");
        assert_eq!(validate_scale_type("PHQ9").unwrap(), "PHQ-9");

        assert_eq!(validate_scale_type("GAD-7").unwrap(), "GAD-7");
        assert_eq!(validate_scale_type("gad7").unwrap(), "GAD-7");

        assert_eq!(validate_scale_type("BDI-II").unwrap(), "BDI-II");
        assert_eq!(validate_scale_type("BDIII").unwrap(), "BDI-II");
        assert_eq!(validate_scale_type("bdi2").unwrap(), "BDI-II");

        assert!(validate_scale_type("INVALID").is_err());
    }

    #[test]
    fn test_phq9_interpretation() {
        assert_eq!(interpret_phq9(0).unwrap(), "Minimal");
        assert_eq!(interpret_phq9(4).unwrap(), "Minimal");
        assert_eq!(interpret_phq9(5).unwrap(), "Mild");
        assert_eq!(interpret_phq9(9).unwrap(), "Mild");
        assert_eq!(interpret_phq9(10).unwrap(), "Moderate");
        assert_eq!(interpret_phq9(14).unwrap(), "Moderate");
        assert_eq!(interpret_phq9(15).unwrap(), "Moderately Severe");
        assert_eq!(interpret_phq9(19).unwrap(), "Moderately Severe");
        assert_eq!(interpret_phq9(20).unwrap(), "Severe");
        assert_eq!(interpret_phq9(27).unwrap(), "Severe");

        assert!(interpret_phq9(-1).is_err());
        assert!(interpret_phq9(28).is_err());
    }

    #[test]
    fn test_gad7_interpretation() {
        assert_eq!(interpret_gad7(0).unwrap(), "Minimal");
        assert_eq!(interpret_gad7(4).unwrap(), "Minimal");
        assert_eq!(interpret_gad7(5).unwrap(), "Mild");
        assert_eq!(interpret_gad7(9).unwrap(), "Mild");
        assert_eq!(interpret_gad7(10).unwrap(), "Moderate");
        assert_eq!(interpret_gad7(14).unwrap(), "Moderate");
        assert_eq!(interpret_gad7(15).unwrap(), "Severe");
        assert_eq!(interpret_gad7(21).unwrap(), "Severe");

        assert!(interpret_gad7(-1).is_err());
        assert!(interpret_gad7(22).is_err());
    }

    #[test]
    fn test_bdi_ii_interpretation() {
        assert_eq!(interpret_bdi_ii(0).unwrap(), "Minimal");
        assert_eq!(interpret_bdi_ii(13).unwrap(), "Minimal");
        assert_eq!(interpret_bdi_ii(14).unwrap(), "Mild");
        assert_eq!(interpret_bdi_ii(19).unwrap(), "Mild");
        assert_eq!(interpret_bdi_ii(20).unwrap(), "Moderate");
        assert_eq!(interpret_bdi_ii(28).unwrap(), "Moderate");
        assert_eq!(interpret_bdi_ii(29).unwrap(), "Severe");
        assert_eq!(interpret_bdi_ii(63).unwrap(), "Severe");

        assert!(interpret_bdi_ii(-1).is_err());
        assert!(interpret_bdi_ii(64).is_err());
    }
}

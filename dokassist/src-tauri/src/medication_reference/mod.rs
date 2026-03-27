pub mod download;

use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::AppError;

/// Compact result used to populate the autocomplete dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceSummary {
    pub id: String,
    pub name_de: String,
    pub atc_code: Option<String>,
    pub trade_names: Vec<String>,
}

/// Full record shown in the detail panel after a substance is selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceDetail {
    pub id: String,
    pub name_de: String,
    pub atc_code: Option<String>,
    pub trade_names: Vec<String>,
    pub indication: Option<String>,
    pub side_effects: Option<String>,
    pub contraindications: Option<String>,
    pub source_version: Option<String>,
}

/// Open the unencrypted reference SQLite located at `db_path`.
/// Creates the schema (substances table + FTS5 index) if the file is new.
pub fn open_reference_db(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AppError::Database)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;

    ensure_schema(&conn)?;
    Ok(conn)
}

fn ensure_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS substances (
            id            TEXT PRIMARY KEY NOT NULL,
            name_de       TEXT NOT NULL,
            atc_code      TEXT,
            trade_names   TEXT,   -- JSON array of strings
            indication    TEXT,
            side_effects  TEXT,
            contraindications TEXT,
            source_version TEXT
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS substances_fts USING fts5(
            name_de,
            trade_names,
            content='substances',
            content_rowid='rowid',
            tokenize='unicode61 remove_diacritics 1'
        );

        -- Keep FTS in sync with the substances table
        CREATE TRIGGER IF NOT EXISTS substances_ai AFTER INSERT ON substances BEGIN
            INSERT INTO substances_fts(rowid, name_de, trade_names)
            VALUES (new.rowid, new.name_de, COALESCE(new.trade_names, ''));
        END;

        CREATE TRIGGER IF NOT EXISTS substances_ad AFTER DELETE ON substances BEGIN
            INSERT INTO substances_fts(substances_fts, rowid, name_de, trade_names)
            VALUES ('delete', old.rowid, old.name_de, COALESCE(old.trade_names, ''));
        END;

        CREATE TRIGGER IF NOT EXISTS substances_au AFTER UPDATE ON substances BEGIN
            INSERT INTO substances_fts(substances_fts, rowid, name_de, trade_names)
            VALUES ('delete', old.rowid, old.name_de, COALESCE(old.trade_names, ''));
            INSERT INTO substances_fts(rowid, name_de, trade_names)
            VALUES (new.rowid, new.name_de, COALESCE(new.trade_names, ''));
        END;",
    )?;
    Ok(())
}

/// FTS5 prefix search — returns up to `limit` matching substances.
pub fn search_substances(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SubstanceSummary>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    // Append '*' for prefix matching; escape double-quotes inside the query.
    let fts_query = format!("\"{}\"*", query.replace('"', "\"\""));

    let mut stmt = conn.prepare(
        "SELECT s.id, s.name_de, s.atc_code, s.trade_names
         FROM substances_fts f
         JOIN substances s ON s.rowid = f.rowid
         WHERE substances_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
        let trade_json: Option<String> = row.get(3)?;
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            trade_json,
        ))
    })?;

    let mut results = Vec::new();
    for row in rows {
        let (id, name_de, atc_code, trade_json) = row?;
        let trade_names = parse_trade_names(trade_json.as_deref());
        results.push(SubstanceSummary {
            id,
            name_de,
            atc_code,
            trade_names,
        });
    }
    Ok(results)
}

/// Fetch full detail for a single substance by its primary key.
pub fn get_substance_detail(conn: &Connection, id: &str) -> Result<SubstanceDetail, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name_de, atc_code, trade_names,
                indication, side_effects, contraindications, source_version
         FROM substances
         WHERE id = ?1",
    )?;

    stmt.query_row(params![id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("medication reference '{}'", id))
        }
        other => AppError::Database(other),
    })
    .map(
        |(
            id,
            name_de,
            atc_code,
            trade_json,
            indication,
            side_effects,
            contraindications,
            source_version,
        )| {
            SubstanceDetail {
                id,
                name_de,
                atc_code,
                trade_names: parse_trade_names(trade_json.as_deref()),
                indication,
                side_effects,
                contraindications,
                source_version,
            }
        },
    )
}

/// Return the `source_version` string stored in the first row of `substances`,
/// or `None` if the DB is empty.
pub fn get_db_version(conn: &Connection) -> Option<String> {
    conn.query_row("SELECT source_version FROM substances LIMIT 1", [], |row| {
        row.get(0)
    })
    .ok()
    .flatten()
}

fn parse_trade_names(json: Option<&str>) -> Vec<String> {
    json.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

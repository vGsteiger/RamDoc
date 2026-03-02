## PKG-6 — Audit Logger

**Goal**: Append-only logging of all data access and modifications for nDSG compliance.

**Depends on**: PKG-2 (database)

**Files**:

```
src-tauri/src/
├── audit.rs
```

**Public interface**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    View,
    Create,
    Update,
    Delete,
    Export,
    LlmQuery,
    Login,
    Logout,
    RecoveryUsed,
}

/// Log an auditable action. Call this from every command that touches patient data.
pub fn log(
    conn: &Connection,
    action: AuditAction,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) -> Result<(), AppError>;

/// Query audit log with filters.
pub fn query_log(
    conn: &Connection,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    from: Option<&str>,       // ISO 8601
    to: Option<&str>,         // ISO 8601
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditEntry>, AppError>;

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
}
```

**Integration pattern**: Every Tauri command calls `audit::log()` before returning:

```rust
#[tauri::command]
async fn get_patient(state: State<'_, AppState>, id: String) -> Result<Patient, AppError> {
    let conn = state.db.conn()?;
    let patient = models::patient::get_patient(&conn, &id)?;
    audit::log(&conn, AuditAction::View, "patient", Some(&id), None)?;
    Ok(patient)
}
```

**Acceptance criteria**:

- [ ] Every patient data access generates an audit entry
- [ ] Audit table has no UPDATE or DELETE operations exposed
- [ ] Log entries contain no PHI (no patient names, only UUIDs)
- [ ] `details` field used for update diffs (field names changed, not values)
- [ ] Query log with date range filtering works
- [ ] Audit log readable from Settings UI

**Effort**: ~4h

-----
## PKG-2 — Database Module (SQLCipher)

**Goal**: Encrypted SQLite database with schema migrations, connection pool, and typed CRUD operations for all entities.

**Depends on**: PKG-1 (needs `db_key` from auth state)

**Files**:

```
src-tauri/src/
├── database.rs           # Connection management, migrations
├── models/
│   ├── mod.rs
│   ├── patient.rs        # Patient struct + CRUD
│   ├── session.rs        # Session struct + CRUD
│   ├── file_record.rs    # File metadata struct + CRUD (not the file itself)
│   ├── diagnosis.rs      # Diagnosis struct + CRUD
│   ├── medication.rs     # Medication struct + CRUD
│   └── report.rs         # Report struct + CRUD
└── migrations/
    └── 001_initial.sql   # Full schema from architecture doc
```

**Public interface**:

```rust
// === database.rs ===

/// Initialize the database connection with SQLCipher encryption.
/// Runs migrations if needed. Returns a connection pool handle.
pub fn init_db(db_path: &Path, key: &[u8; 32]) -> Result<DbPool, AppError>;

/// Wrapper around r2d2 + rusqlite connection pool.
pub struct DbPool { /* ... */ }

impl DbPool {
    pub fn conn(&self) -> Result<PooledConnection, AppError>;
}


// === models/patient.rs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: String,
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePatient {
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
}

pub fn create_patient(conn: &Connection, input: CreatePatient) -> Result<Patient, AppError>;
pub fn get_patient(conn: &Connection, id: &str) -> Result<Patient, AppError>;
pub fn update_patient(conn: &Connection, id: &str, input: UpdatePatient) -> Result<Patient, AppError>;
pub fn delete_patient(conn: &Connection, id: &str) -> Result<(), AppError>;
pub fn list_patients(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<Patient>, AppError>;
```

**Same CRUD pattern for**: `Session`, `FileRecord`, `Diagnosis`, `Medication`, `Report`

**Tauri commands** (`commands/patients.rs`, etc.):

```rust
#[tauri::command]
async fn create_patient(state: State<'_, AppState>, input: CreatePatient) -> Result<Patient, AppError>;

#[tauri::command]
async fn get_patient(state: State<'_, AppState>, id: String) -> Result<Patient, AppError>;

#[tauri::command]
async fn list_patients(state: State<'_, AppState>, limit: u32, offset: u32) -> Result<Vec<Patient>, AppError>;

// ... update, delete for each entity
```

**Migration system**: Simple sequential SQL files. On `init_db`, check a `schema_version` pragma, run any unapplied migrations in order.

**AHV number validation** (built into `create_patient`):

```rust
/// Validates Swiss AHV/AVS number format: 756.XXXX.XXXX.XX
/// Accepts both dotted and plain 13-digit formats.
pub fn validate_ahv(ahv: &str) -> Result<String, AppError>;
```

**Acceptance criteria**:

- [ ] Database opens only with correct key; wrong key returns error, not garbage data
- [ ] All migrations run idempotently
- [ ] Patient CRUD: create → read → update → delete works end-to-end
- [ ] AHV validation rejects invalid formats, normalizes to dotted format
- [ ] UUIDv7 IDs are time-sortable
- [ ] Connection pool handles concurrent reads from Tauri async commands
- [ ] All entity types have working CRUD operations
- [ ] `updated_at` auto-updates on modifications

**Effort**: ~16h

-----
## PKG-5 — Search Engine (FTS5)

**Goal**: Unified full-text search across patients, files, sessions, and reports.

**Depends on**: PKG-2 (database must exist with FTS5 virtual table)

**Files**:

```
src-tauri/src/
├── search.rs         # FTS5 indexing + querying
```

**Public interface**:

```rust
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub result_type: String,     // "patient", "file", "session", "report"
    pub entity_id: String,
    pub patient_id: String,
    pub patient_name: String,
    pub title: String,           // display title for result
    pub snippet: String,         // highlighted match context
    pub date: Option<String>,
    pub rank: f64,
}

/// Full-text search across all indexed content.
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<SearchResult>, AppError>;

/// Index or re-index a patient's searchable fields.
pub fn index_patient(conn: &Connection, patient: &Patient) -> Result<(), AppError>;

/// Index file content (called after LLM metadata extraction).
pub fn index_file(conn: &Connection, file: &FileRecord, extracted_text: &str) -> Result<(), AppError>;

/// Index session notes.
pub fn index_session(conn: &Connection, session: &Session) -> Result<(), AppError>;

/// Index finalized report content.
pub fn index_report(conn: &Connection, report: &Report) -> Result<(), AppError>;

/// Remove all index entries for an entity.
pub fn remove_from_index(conn: &Connection, entity_type: &str, entity_id: &str) -> Result<(), AppError>;
```

**AHV search normalization**:

```rust
/// Normalize AHV queries: "7561234567897" and "756.1234.5678.97" both match.
fn normalize_ahv_for_search(query: &str) -> String;
```

**Tauri commands** (`commands/search.rs`):

```rust
#[tauri::command]
async fn global_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, AppError>;
```

**Acceptance criteria**:

- [ ] Search by patient name returns correct patients (partial match, case-insensitive)
- [ ] Search by AHV number works in both dotted and plain formats
- [ ] Search by file content returns files with matching extracted text
- [ ] Results ranked by relevance (FTS5 rank)
- [ ] Snippets contain `<mark>` tags around matched terms
- [ ] Unicode/German characters handled correctly (ä, ö, ü, ß)
- [ ] Diacritics-insensitive: searching "muller" matches "Müller"
- [ ] Re-indexing updates results (not duplicates)
- [ ] Search returns in < 50ms for databases with 1000+ patients

**Effort**: ~8h

-----
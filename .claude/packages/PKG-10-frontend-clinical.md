## PKG-10 — Frontend: Clinical (AMDP, Sessions, Diagnoses, Medications)

**Goal**: The clinical workflow screens — where the doctor does actual psychiatry work.

**Depends on**: PKG-2, PKG-7

**Files**:

```
src/routes/patients/[id]/
├── sessions/
│   ├── +page.svelte           # Session list
│   └── new/
│       └── +page.svelte       # New session form with AMDP
├── diagnoses/
│   └── +page.svelte           # Diagnosis list + ICD-10 search
├── medications/
│   └── +page.svelte           # Medication list + add/edit
src/lib/components/
├── AMDPForm.svelte             # AMDP psychopathological findings (12 categories)
├── AMDPCategory.svelte         # Single AMDP category with 0-3 scoring buttons
├── IcdSearch.svelte            # ICD-10-GM typeahead search
├── MedicationForm.svelte
├── SessionCard.svelte
└── DiagnosisCard.svelte
```

**AMDP form structure** (12 categories, ~140 items):

```typescript
interface AMDPCategory {
    name: string;           // e.g. "Bewusstsein"
    items: AMDPItem[];
}

interface AMDPItem {
    code: string;           // e.g. "Bew-1"
    label: string;          // e.g. "Bewusstseinsverminderung"
    score: 0 | 1 | 2 | 3;  // not present | mild | moderate | severe
}
```

Scores stored as JSON blob in `sessions.amdp_data`.

**ICD-10-GM search**: Load `static/icd10gm.json` at startup. Typeahead component searches by code and description. Data source: free XML from BfArM (formerly DIMDI), pre-converted to JSON at build time.

**Acceptance criteria**:

- [ ] New session form with free-text notes and AMDP scoring
- [ ] AMDP form: all 12 categories navigable, 0-3 tap scoring, scores persist
- [ ] Session list shows date, type, duration, and a summary snippet
- [ ] ICD-10 search returns results as user types (< 50ms, client-side)
- [ ] Add/remove diagnoses with status (active/remission/resolved)
- [ ] Medication list with substance, dose, frequency, date range
- [ ] All clinical data saves to SQLCipher and is searchable

**Effort**: ~18h

-----
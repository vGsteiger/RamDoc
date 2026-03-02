## PKG-8 — Frontend: Patient Management

**Goal**: Patient list, detail view, create/edit forms.

**Depends on**: PKG-2, PKG-5, PKG-7

**Files**:

```
src/routes/
├── patients/
│   ├── +page.svelte              # Patient list with search
│   ├── [id]/
│   │   ├── +page.svelte          # Patient detail (tabs)
│   │   └── +layout.svelte        # Tab navigation for patient subpages
│   └── new/
│       └── +page.svelte          # Create patient form
src/lib/components/
├── PatientCard.svelte             # List item card
├── PatientForm.svelte             # Create/edit form (shared)
├── AhvInput.svelte                # AHV number input with formatting/validation
└── PatientTabs.svelte             # Tab bar: Overview | Sessions | Files | Diagnoses | Meds | Reports
```

**Patient list features**:

- Search bar (calls `global_search` — filters to patients)
- Sort by: last name, last visit, created date
- AHV number displayed in formatted form
- Click to navigate to detail view

**Patient detail tabs**: Overview, Sessions, Files, Diagnoses, Medications, Reports
(each tab's content is built in PKG-9a, PKG-10, PKG-11)

**AHV input component**: Auto-formats as user types (`756.____.____.__ `), validates checksum.

**Acceptance criteria**:

- [ ] Create patient with all fields, AHV validated
- [ ] Patient list loads and displays correctly
- [ ] Search filters patient list in real-time (debounced, < 100ms perceived)
- [ ] Edit patient details, changes persist
- [ ] Delete patient with confirmation dialog
- [ ] Tab navigation on detail page works
- [ ] AHV input auto-formats and validates

**Effort**: ~12h

-----
# DokAssist — In-Depth Usability Study

**Date:** 2026-03-21
**Scope:** Full application, all routes and workflows
**Method:** Static analysis of frontend components (26 Svelte components, 20+ routes) and backend commands (20 modules, 12 data models, 9 DB migrations)

---

## 1. Onboarding & Authentication

### 1.1 Initial Setup (`/setup`)

**Issue — cognitive load of mnemonic verification**
The setup shows 24 BIP-39 words, then asks users to confirm 3 random words. For a non-crypto-native clinical audience, this pattern is unfamiliar and anxiety-inducing. The yellow "store these safely" warning lacks actionable guidance (print? write on paper? password manager?).

**Issue — no progress indicator**
The two-step flow (display → confirm) has no numbered steps or progress bar. Users don't know if there are more steps after confirmation.

**Issue — no recovery phrase format guidance**
After writing down 24 words, users close the app and must re-open months later. There's no hint that word order matters, or that diacritics/case must be preserved.

**Recommendation:** Add a numbered stepper (Step 1 of 2), a concrete "Write this on paper and store in a locked cabinet" instruction, and a note that word order matters.

---

### 1.2 Unlock (`/unlock`)

**Strength:** Touch ID unlock is fast and familiar. The "Welcome Back" heading provides reassuring context.

**Issue — factory reset is too prominent**
The factory reset section (destructive, irreversible) is visible on the unlock page — the page visited most frequently. A fatigued or rushed user could trigger it by mistake. The two confirmation buttons ("Yes, wipe everything" / "Cancel") are co-located without a required text input (e.g., typing "DELETE").

**Recommendation:** Move factory reset to Settings → Danger Zone, behind at least one extra navigation step. Add a typed confirmation before irreversible actions.

---

### 1.3 Recovery (`/recover`)

**Issue — no word-by-word validation**
24 inputs accept any text. Users who mistype a word only discover the error on final submission. A BIP-39 wordlist check per-field (showing green/red) would catch typos early.

**Issue — no paste support**
Users who stored words in a password manager as a single string can't easily paste them. A "Paste all words" button that splits on whitespace would significantly reduce friction.

**Recommendation:** Add per-field BIP-39 validation (not blocking, just indicative) and a "Paste all 24 words" utility.

---

## 2. Navigation & Information Architecture

### 2.1 Sidebar

**Strength:** 6 clear items with icons + labels is within cognitive limits. Active state (blue highlight) is unambiguous. Lock button at bottom is well-placed.

**Issue — Literature is a dead end**
The `/literature` route is in primary navigation but its content is unexplored/underdeveloped. Placing an incomplete feature in primary nav creates confusion ("why does nothing happen here?").

**Issue — no breadcrumbs in deep routes**
When a user is at `/patients/abc123/sessions/def456`, there's no breadcrumb to indicate nesting depth or allow jumping to `/patients/abc123`. The back button relies on browser history which can be lost.

**Recommendation:** Add a breadcrumb strip below the top bar for patient-scoped routes: `Patients > Müller, Hans > Sessions > Session vom 21.03.2026`. Remove Literature from primary nav until the feature is ready.

---

### 2.2 Patient Detail Tabs

**Issue — Sessions, Medications, Diagnoses, Treatment Plans are missing from patient tabs**
The tabs (Overview, Files, Reports, Email, Chat) don't include Sessions, Medications, Diagnoses, or Treatment Plans. Those features live under separate routes not linked from the patient layout. This creates confusion: "Where are my session notes?"

**Issue — Sessions vs. Calendar ambiguity**
Sessions exist both under `/calendar` (global view) and under `/patients/[id]/sessions`. A user who creates a session from the Calendar can't easily navigate back to that patient's session history without knowing the separate route.

**Recommendation:** Add Sessions (and optionally Medications/Diagnoses) as tabs in the patient detail layout.

---

### 2.3 Global Search (TopBar)

**Strength:** Cmd+K shortcut, debounced search, 6 result types with type badges are well-designed and powerful.

**Issue — search result snippets lack date context**
A session result shows patient name and a text snippet, but not the session date. Clinical users searching across hundreds of sessions need dates to distinguish results.

**Issue — search scope is opaque**
There's no indication of what entity types are indexed. A user who searches for a medication name and gets no result doesn't know if it's because the medication doesn't exist or because medications aren't in the search index.

**Recommendation:** Add date context to session and report results. Add a brief "Searching patients, sessions, files, reports..." hint in the dropdown.

---

## 3. Dashboard (`/dashboard`)

**Strength:** The 3-panel layout (today's sessions, recent patients, incomplete notes) directly maps to the clinical morning workflow. "Incomplete notes" is particularly valuable as a quality assurance aid.

**Issue — "Incomplete notes" definition is too strict**
A session with `notes = NULL OR notes = ''` is flagged as incomplete. A session with a single space (`' '`) is not flagged. More importantly, very brief notes ("See AMDP data") would not be flagged even if clinically insufficient — creating false confidence.

**Issue — no last-seen date on recent patients**
The "Recent Patients" panel shows 5 patients by last session activity, but doesn't display when they were last seen. A user returning after a vacation can't quickly tell which patients need follow-up.

**Issue — no weekly summary statistics**
There's no "patients seen this week", "upcoming sessions", or "reports due" count. Clinical staff tracking throughput lack data they need.

**Recommendation:** Add last-seen date to Recent Patients cards. Add a summary row with this-week session count and upcoming sessions in the next 7 days.

---

## 4. Patient Management

### 4.1 Patient Form

**Strength:** 12 fields are logically ordered. Required fields (AHV, first/last name, DOB) are marked. AhvInput with EAN-13 checksum validation prevents data entry errors.

**Issue — AHV duplicate gives a DB error, not a friendly message**
AHV uniqueness is enforced at the DB level. A user who accidentally starts entering a duplicate patient sees a cryptic error instead of "A patient with this AHV number already exists."

**Issue — address is a single textarea**
Stored as free text, making it impossible to filter patients by city/canton or generate envelopes. Clinical software typically uses structured fields (street, ZIP, city, canton).

**Issue — no emergency contact field**
Standard clinical intake requires an emergency contact. This field is absent.

**Recommendation:** Check AHV duplication client-side before submission. Consider structured address fields. Flag emergency contact as a future addition.

---

### 4.2 Patient List

**Strength:** Search + sort (name/created) is functional. PatientCard shows AHV, DOB, gender, insurance.

**Issue — no filter by status or insurance**
A clinic managing 100+ patients can't filter by insurance type, active/inactive status, or treating therapist — making billing and reporting workflows manual.

**Issue — PatientCard has no quick-action menu**
Common actions (new session, view last report) require navigating into the patient detail first. A hover overflow menu would save clicks.

**Issue — no pagination**
If the query returns all patients, performance degrades at scale. The UI doesn't paginate or virtualize.

---

## 5. Calendar (`/calendar`)

### 5.1 Week View

**Strength:** 7-day grid with hourly slots (07:00–21:00), colored session blocks, and status legend provides a solid clinical day-view.

**Issue — session creation flow is inverted**
Creating a session from a time slot navigates to `/patients/new?date=...`, requiring the user to select a patient first. Most scheduling tools start from the time slot and ask for the patient second — the current flow is backwards.

**Issue — no "Mark as Completed" action on calendar cards**
Sessions stay in "scheduled" state indefinitely. There's no one-click completion workflow from the calendar itself.

**Issue — no drag-to-reschedule**
Rescheduling requires opening the session, editing date/time, and saving. Drag-and-drop is the expected UX for scheduling tools.

**Issue — 1-hour granularity misrepresents 50-min sessions**
Sessions are typically 50 minutes (the clinical hour). Back-to-back sessions starting at :00 and :50 don't display cleanly in an hourly grid.

**Recommendation:** Add a "New Session at this time" modal (patient picker + quick fields) triggered from time-slot click. Add a "Mark Completed" button on session cards. Consider 30-min granularity.

---

### 5.2 Month View

**Issue — dots don't show session counts**
A day with 5 sessions shows the same as a day with 1 session. The 3-dot max truncates without indicating overflow.

**Recommendation:** Show a session count badge on each day cell.

---

## 6. Session Management

**Strength:** AMDP psychopathology form integration is a significant clinical differentiator — structured assessment built directly into session documentation.

**Issue — session type is not validated**
Session type appears to be free text or a free select. Without a fixed vocabulary, search and reporting are unreliable (e.g., "Erstgespräch" vs "Erstgesprach" won't match).

**Issue — notes required even with full AMDP data**
If a clinician fills in the complete AMDP form (40+ items), requiring separate notes creates double documentation burden.

**Issue — outcome scores not on session creation form**
PHQ-9/GAD-7/BDI-II scores are entered separately on the session detail page. Clinical practice typically administers these during the session — they should be on the creation form.

**Recommendation:** Add session type as a localised enum select. Make notes optional when AMDP data is present. Add an optional outcome scores section to session creation.

---

## 7. Diagnoses & Medications

### 7.1 Diagnoses

**Strength:** ICD-10 code search is clinically essential and well-integrated.

**Issue — no temporal visualization**
Diagnoses have `diagnosed_date` and `resolved_date`, but the list doesn't render a timeline or make active vs. resolved visually distinct beyond a status text field.

**Issue — duplicate ICD codes possible**
No deduplication check — a clinician could add F32.1 twice without warning.

---

### 7.2 Medications

**Issue — no drug interaction disclaimer**
Medication records exist but there's no reference to interactions. At minimum, a "This app does not check for drug interactions" disclaimer should be visible.

**Issue — current medications not surfaced in patient overview**
Active medications (where `end_date IS NULL`) aren't shown on the patient overview tab — a clinician must navigate to the medications tab separately.

---

## 8. Reports

**Strength:** AI-assisted report streaming is a core differentiator. Real-time token streaming confirms to users that generation is progressing.

**Issue — model must be manually loaded from Settings**
If the LLM model isn't loaded, users must navigate to Settings → load the model → return to the patient → retry. This multi-step interruption is a significant friction point in clinical workflows.

**Issue — report types are not described in-app**
The ReportTypeSelector shows available types but doesn't describe what each type generates or what input is required. A new user creating their first discharge summary has no guidance.

**Issue — no version history**
Reports can be edited after generation, but edits permanently overwrite the original. There's no way to compare versions or recover a previous draft.

**Recommendation:** Auto-load the model in the background when navigating to chat or reports. Add report type tooltips/descriptions. Consider auto-saving drafts.

---

## 9. Chat / AI Agent

**Strength:** Patient-scoped chat is thoughtful — context is loaded per patient. Streaming responses with collapsible tool calls give transparency into AI actions.

**Issue — tool results are raw JSON**
Tool call results are displayed as unformatted JSON (e.g., `{ "patient_id": "abc", "first_name": "Hans", ... }`). Clinical users don't need raw JSON — they need natural language summaries.

**Issue — no suggested prompts for new users**
An empty chat input with no guidance leaves new users uncertain how to start. Suggested starters ("Summarize this patient's history", "Draft a session note for today") would lower the barrier significantly.

**Issue — chat disabled without a loaded model**
The model-not-loaded state degrades to a disabled input with a banner, but doesn't offer to queue the download or navigate to settings. The banner should include a direct action button.

**Recommendation:** Render tool results as formatted text instead of raw JSON. Add 3-5 suggested prompts in the empty chat state. Add a "Download Model" shortcut to the banner.

---

## 10. File Management

**Strength:** Drag-and-drop with progress, async OCR/embedding, and encrypted vault storage are technically sound.

**Issue — FileViewer behavior is opaque**
Viewing encrypted files likely requires decrypting to a temp location. The UI doesn't indicate this or explain the lifecycle of temp files after viewing.

**Issue — no batch delete**
Removing multiple files requires individual confirmation dialogs. There's no "Select all → Delete selected" workflow.

---

## 11. Settings

**Strength:** Theme, language (de/en), LLM model management, update checking, and data export are all present.

**Issue — LLM management is disconnected from clinical workflows**
Settings is the only place to download and load models. A clinical user who hits a "model not loaded" state mid-workflow must leave, navigate to Settings, manage the model, and return.

**Issue — no in-app help or About page**
There's no documentation link, version info screen, or onboarding guide beyond the initial setup wizard.

**Issue — data export format is undocumented**
The export option doesn't describe what is exported (all data? patients only? files?), in what format, or how to restore from it.

---

## 12. Error Handling & Feedback

**Strength:** Structured error display with ref codes, copy-to-clipboard, and expandable technical details is best-in-class for a clinical app. This pattern enables remote support without exposing sensitive data.

**Issue — no success toasts**
After creating a patient, saving a session, or uploading a file, feedback is implicit (redirect or item appears in list). In workflows with multiple rapid actions, a brief success toast would confirm each action.

**Issue — inconsistent loading states**
Some operations show a spinner + text ("Saving..."), others just disable the button. A consistent pattern would reduce uncertainty.

**Issue — rate-limit error wording is technical**
The recovery brute-force protection returns a technical "wait X seconds" message. The wording should be user-friendly: "Too many attempts. Please wait 5 minutes before trying again."

---

## 13. Accessibility

**Gaps identified:**
- Icon-only buttons (copy, delete, expand) lack `aria-label` attributes
- Calendar session status uses color only (blue/green/amber) — no text labels for color-blind users
- No `aria-describedby` linking form validation errors to their inputs
- No skip-to-main-content link for keyboard navigation
- Mobile sidebar behavior undefined (no hamburger menu or touch drawer)

**Recommended priority:**
1. Add `aria-label` to all icon-only buttons
2. Add visible text labels to calendar status indicators ("Scheduled", "Completed", "Pending Notes")
3. Associate form errors with inputs via `aria-describedby`

---

## 14. Internationalization (de/en)

**Issue — date/time formatting is inconsistent**
Some dates render as "21.03.2026" (Swiss/German format), others as "2026-03-21" (ISO). A consistent locale-aware formatter should be used throughout.

**Issue — silent English fallbacks**
If any component lacks a German translation key, it silently falls back to English — creating a mixed-language UI that looks like a bug rather than a design decision.

---

## 15. Summary: Top 10 Prioritized Issues

| # | Issue | Impact | Effort |
|---|---|---|---|
| 1 | Sessions/Medications/Diagnoses missing from patient tabs | High | Medium |
| 2 | Calendar session creation flow is inverted (slot → patient) | High | Medium |
| 3 | LLM model must be manually loaded — breaks clinical workflows | High | Low |
| 4 | Factory reset on unlock screen — too accessible, too destructive | High | Low |
| 5 | No success toasts — implicit feedback insufficient | Medium | Low |
| 6 | Calendar: no "Mark Completed" action on session cards | Medium | Low |
| 7 | Chat tool results shown as raw JSON | Medium | Low |
| 8 | AHV duplicate gives DB error, not friendly message | Medium | Low |
| 9 | Accessibility: no aria-labels, color-only calendar status | Medium | Low |
| 10 | No emergency contact / structured address in patient form | Low | Medium |

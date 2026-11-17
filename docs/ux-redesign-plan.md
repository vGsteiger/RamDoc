# RamDoc UX redesign plan

Status: proposal, audit + plan only. Nothing here is implemented yet.
Scope: the SvelteKit frontend in `dokassist/src/` (routes, components, tokens).
Builds on: #428 / PR #429 (token-driven design language), PR #484 (chat as the
primary clinical workspace), `docs/design-language.md`, `docs/i18n.md`,
`docs/usability-study.md`.

How this was produced: a static review of every route and component, the
baseline checks (`check:design` and `check:i18n` pass; `svelte-check` 0/0;
ESLint 69 errors and 7 warnings already present on `main`), and rendered
screenshots of the app in Chrome. The screenshots used a throwaway mock of the
Tauri IPC (`window.__TAURI_INTERNALS__`, injected by Playwright; no app source
changed) at 700, 1024, 1440 and 1920 px. See §10 for what was rendered and what
was only inferred.

Line numbers refer to `main` at `a24a3bc`.

The decisions the product owner needs to make are in §9. Three of them block
the foundation workstream: D1 accent, D2 navigation and D5 density.

---

## 1. Screen inventory

| Route | Primary job | Notes |
| --- | --- | --- |
| `/` | Auth gate, redirects to `/chat` (or onboarding) | Layoutless |
| `/setup`, `/unlock`, `/recover`, `/reset` | Vault creation, unlock, mnemonic recovery, factory reset | Layoutless; safety-critical |
| `/onboarding/step1..4` | Practice details, model download, data import, feature tour | Layoutless |
| `/chat` | **Primary workspace** (PR #484): chat list, context picker, thread | Default landing |
| `/dashboard` | Today's sessions, recent patients, incomplete notes | Still a nav item |
| `/patients`, `/patients/new` | Patient list with search and sort; create | |
| `/patients/[id]` (+ layout) | Patient header, 9 tabs, overview (history query, trends, demographics) | |
| `…/sessions`, `…/sessions/new`, `…/sessions/[sid]` | Session list, AMDP entry, session note, summary and report stream | |
| `…/diagnoses`, `…/medications`, `…/treatment-plans` | Structured clinical data | treatment-plans is 958 lines |
| `…/files`, `…/reports(/[rid])`, `…/email(/new,/[eid])`, `…/letters/new` | Documents and correspondence | No letters list; letters are not reachable from the tabs |
| `…/chat`, `…/reports/new` | Redirect stubs into `/chat` | Still shown as a tab |
| `/calendar` | Week and month view of sessions | |
| `/literature` | Medication reference search plus uploaded literature RAG | |
| `/settings` | Everything else (2018 lines, one scroll) | |

Shell: `Sidebar` (6 flat items + Lock), `TopBar` (global search + "LLM" status
dot), `CommandPalette` (Cmd+K), `Toast`. `PatientTabs.svelte`,
`ReportTypeSelector.svelte` and `UpdateNotification.svelte` are dead code:
nothing imports them.

---

## 2. Findings

Priorities: **P0** is broken or unsafe, **P1** is a structural or IA problem
that blocks a coherent experience, **P2** is inconsistency or polish.

### 2.1 P0: broken or unsafe

1. **Settings: "Check for Updates" overflows its button.** Reproduced in the
   render at the default window width of 1280px, in both en and de, and in
   de up to at least 1366px. The button's content is 43px tall inside a
   fixed `h-8` (32px) box. The cause has two parts:
   - The button (`settings/+page.svelte:624-630`) has no `whitespace-nowrap`
     or `shrink-0`.
   - From 1280px up, the scoped `<style>` grid (`:1947-2010`) places sections
     by **`nth-of-type` position**. It gives the updates section 4 of 12
     columns, a card about 309px wide, so the label wraps.

   The positional CSS is fragile: adding or reordering a section silently
   re-lays out the page. None of Settings' 23 raw `<button>`s use `Button`,
   which already sets `whitespace-nowrap`.
1a. **Chat renders assistant markdown as raw text.** `ChatMessage.svelte:288`
   uses `whitespace-pre-wrap` with no markdown rendering, so replies show
   `**Schlaf**` and `- ` literally. Rendered.
1b. **Search highlighting is broken in both search UIs.** The backend emits
   `<mark>` (`src-tauri/src/search.rs:81`). The TopBar prints the tags as
   literal text (`TopBar.svelte:193`). The CommandPalette's
   `sanitizeSnippet` (`CommandPalette.svelte:367-369`) mangles them and
   **deletes the matched word**. Rendered.
1c. **Calendar week view has no weekday or date header**
   (`calendar/+page.svelte:327-342`). Events are fixed-height and truncated
   ("Anna …") at 700–1024px. There is no today marker, and the colour has no
   legend. Dates are hardcoded to `de-CH` (`:58,59,122`) even in the English
   UI. Rendered.
1d. **Wrong unit and label text.**
   - The session header shows "50 Duration (minutes)", because the label is
     reused as the unit (`sessions/[sessionId]/+page.svelte:366`).
   - The sessions list shows "50 Min." in English.
   - The ICD search placeholder is hardcoded German (`IcdSearch.svelte:14`).
   - There are three date formats on one dashboard: "March 12, 1984",
     "12.03.1984" and "1984-03-12".
2. **Settings: invalid utility classes fail silently.** `text-md` is not in
   the scale, so these headings fall back to the body size:
   `settings/+page.svelte:1071,1328` and `treatment-plans/+page.svelte:606,787`.
   `check:design` does not catch unknown sizes.
3. **No-op hovers.** Hover is identical to the base state in
   `bg-success hover:bg-success` (`settings:683,1599`),
   `bg-warning hover:bg-warning` (`settings:1748,1775,1831,1857`) and
   `bg-surface-selected hover:bg-surface-selected` (`settings:1076,1183,1785,1867,1933`;
   `calendar:303,312`). The theme and language radio cards
   (`settings:718-841`) put `hover:bg-surface-hover` on a `bg-surface-hover`
   parent, so they also never change on hover.
4. **Destructive actions are placed next to everyday ones.** "Delete patient"
   is a solid danger button in the same action row as Edit and Export
   (`patients/[id]/+page.svelte:282-287`). Its confirmation is a hand-rolled
   modal with no focus trap or focus return (`:481-524`), while `ui/Dialog`
   provides both. Nine deletions use the native `window.confirm()`
   (medications:60, diagnoses:77, treatment-plans:132/216/290, files:64,
   email:29, reports:27, reports/[rid]:60). The native dialog is unstyled,
   can't name the record, and breaks the app's visual language exactly where
   care matters most.
5. **Hardcoded English in German UI:**
   `'measurement' : 'measurements'` at `patients/[id]/+page.svelte:330`.
   `check:i18n` misses string literals inside `{…}` expressions.
6. **Model status is an 8×8px button.** `TopBar.svelte:212-230`. This is
   below WCAG 2.5.8's 24×24 minimum. The colour (red, amber or green) is the
   only signal next to a bare "LLM" label (`:204`).

### 2.2 P1: information architecture and layout

7. **Chat-first is declared, but the navigation still treats chat as the
   fifth item.** `Sidebar.svelte:27-34` orders Dashboard, Patients, Calendar,
   Literature, Chat, Settings. `/` redirects to `/chat`, but the nav says
   Dashboard is home.
8. **Active nav state breaks on nested routes.** `Sidebar.svelte:110` tests
   `currentPath === item.path`. On `/patients/abc/sessions` no item is
   highlighted, so the user loses their place in exactly the deep routes the
   usability study flagged (§2.1).
9. **There are three navigation columns before any content.** The `/chat`
   page adds its own 256px chat-list column (`chat/+page.svelte:114-131`)
   beside the 224px app sidebar. At a 700px window that leaves the thread
   about 388px; at 1024 about 544px. Below `sm` (640px, which the window
   cannot reach) the list stacks above the thread at a fixed `h-52`.
10. **The context picker pushes the thread down.** `chat/+page.svelte:160-174`
    renders it inline above the thread. At the 600px minimum height an open
    picker leaves almost no room for the conversation.
11. **The patient workspace has 9 tabs** (`patients/[id]/+layout.svelte:18-32`),
    including a "Chat" tab that is only a redirect stub (`…/chat/+page.svelte`)
    and an Email tab, while Letters (`letters/new`) has no tab or list at all.
    The tab strip scrolls horizontally at 700–1024px. Each subpage then adds
    its own `<h1>` and `p-8` under the patient header, which stacks two page
    titles and drops the content well below the fold.
12. **The patient header is inconsistent.** It shows the raw ISO date of
    birth (`+layout.svelte:71`) while the overview shows `formatDate`
    (`+page.svelte:369`). NISTIR 7804-1 recommends a reserved, consistent area
    for patient identification. The header should carry the same
    identification fields (name, DOB, AHV) everywhere, chat included.
13. **The patient overview is a form dump.** Edit is the primary (accent)
    action (`+page.svelte:227-232`). Demographics are rendered as label/value
    pairs at full `text-body` weight, below two charts of about 400px each.
    The "Timeline" section (`:291-299`) is actually an "Ask about this
    patient" panel (`PatientHistoryQuery`), a **second, separate chat
    interface** with its own composer and thinking selector. It duplicates
    `/chat` instead of handing off to it. Edit (blue), Send Email (grey) and
    Delete (red) carry equal visual weight. In the render, about 200px of
    fixed headers sit above the content on every patient tab: a third of a
    700×600 window. Each tab also renders two display-size titles. Content
    width and left edge differ per tab:
    - Sessions and Medications are full width.
    - Diagnoses is about 832px, centred.
    - Overview is a card of about 896px, centred.
    - Treatment plans starts at a different x.
11a. **`letters/new` is orphaned.** Nothing links to it, and its "Back to
    Letters" button goes to the overview.
14. **Settings is a 2000-line single scroll with no structure.** It has 11
    sections (updates, appearance, model management, embedding model,
    medication reference, CSV import, backup, about, danger zone). There is
    no sub-navigation, the version is shown twice (updates and about), and
    the expert diagnostics (KV cache, batch sizes, router residency) sit at
    the same level as theme. **Practice details cannot be edited after
    onboarding:** `updateSettings` is only called from `onboarding/step1`.
    The header uses an accent-coloured uppercase "RamDoc" eyebrow
    (`settings:604`), which breaks the accent-scarcity rule. At
    `max-w-[1600px]` (`:601`), lines run past 200 characters at 1920px.
15. **There is no shared page shell.** Eight different container recipes
    exist (`p-8`, `p-4 sm:p-6 lg:p-8`, `p-6`, `px-5 py-6 sm:px-8 lg:px-10`;
    `max-w-3xl/4xl/5xl/6xl/7xl/[1600px]`/none). Page titles are sometimes
    `<h2 class="text-display">` (email:61, reports:55, email/new:206).
    `PageHeader` is used on only 4 routes.
16. **The dashboard duplicates the home.** Its three panels are the
    clinician's morning context, but they live on a page that is neither the
    landing page nor linked from chat.
17. **Literature mixes two things.** The medication reference (structured
    lookup) and uploaded literature (RAG) share a page and a nav slot labelled
    "Literature". The usability study already calls it a dead end.
18. **The brand mark is absent.** The app never shows the ram glyph. The
    sidebar shows the word "RamDoc", or "R" when collapsed
    (`Sidebar.svelte:102-103`). `dokassist/app-icon.svg` is an obsolete "DA"
    placeholder.

### 2.3 P2: system consistency

19. **The primitives are barely adopted.** There are 202 raw `<button>`,
    54 `<input>`, 15 `<select>` and 26 `<textarea>` elements in routes and
    components. `Button` is imported by 7 files, `Dialog` by 1, and
    `IconButton`, `Kbd` and `Skeleton` by 0.
20. **The accent is not scarce.** There are 87 accent-filled elements in 36
    files: 14 in Settings alone, 7 in treatment plans, and 4 in session
    detail. Chat has two accent buttons (New chat, Send), accent user bubbles
    (`ChatMessage.svelte:269`), an accent-subtle banner and accent chips. The
    calendar view toggle uses a solid accent fill (`calendar:280-295`).
    Badges are filled (`settings:1093-1101`, `bg-accent`/`bg-success` with
    on-colour text).
21. **Control heights have drifted from the documentation.** The docs say
    32px default and 28px dense. PR #484 raised `Button` to `min-h-10`/`min-h-9`
    (40/36px) for "touch targets", but raw buttons stayed at `h-8`/`h-7`, and
    auth and onboarding use `px-6 py-3` (about 48px). Counts: h-7 ×34, h-8 ×90,
    min-h-9 ×4, min-h-10 ×14, h-10 ×7, h-12 ×10, `px-6 py-3` ×18,
    `px-4 py-3` ×19. One screen can mix three button heights.
22. **Scrims are inconsistent and bypass the token rule.** Dialog uses
    `bg-black/25` (allow-listed), the Sidebar `bg-black/25`, and the
    CommandPalette and patient delete modal `bg-black/50`. `check:design`'s
    `RAW_NEUTRAL` regex skips `bg-black/NN` because of its `(?![\w/-])`
    lookahead.
23. **Status colours carry meaning without text.** Examples: the model dot,
    the embedding dot (`settings:1394-1400`), calendar status
    (usability §13), and `opacity-60` for "cannot run model"
    (`settings:1230`).
24. **Empty, loading and error states are improvised per page.** Loading is
    `ListSkeleton` on 4 routes, a 48px hourglass icon in the patient layout,
    a `border-b-2` spinning div in setup, and plain "Loading…" text
    elsewhere. Errors are `Alert`, `ErrorDisplay`, or hand-rolled danger
    boxes. Only 2 routes use `EmptyState`. The chat empty state is an
    underlined text link (`chat/+page.svelte:190-198`) with no suggested
    prompts (usability §9).
25. **Hover-only actions.** Chat-list rename and delete appear only on
    `group-hover` (`ChatSessionList.svelte:130`). They can't be discovered by
    keyboard or found without pointing at the row.
26. **Inline SVG chevrons duplicate lucide icons:** `patients/[id]:249`,
    `medications:239,254`, and 7 in treatment plans.
27. **Search exists twice.** The TopBar search (`TopBar.svelte:150-201`) and
    the CommandPalette (Cmd+K) both search patients. The TopBar spends a full
    56px row on it at every width.

### 2.4 Responsiveness summary

The window minimum is 700×600. Breakpoints in use are `sm` (640) and a
custom `max-[900px]` rail for the sidebar. There is no `lg`/`xl`/`2xl`
strategy for wide screens.

Measured in the render: no route scrolls the page horizontally at any
width. The only text-overflowing button anywhere is "Check for Updates"
(finding 1).

- **700px:** sidebar rail (56px) + chat list (256px, can't be collapsed)
  leaves about 330px of usable thread. The patient tab strip scrolls with no
  affordance: Reports, Email and Chat are off-screen. `p-8` page padding
  takes 64px. The calendar's 8-column week grid shrinks each day to about
  70px. The mobile drawer in `Sidebar.svelte` can't be reached at all: the
  toggle is `sm:hidden` and the window minimum is 700px, so that code is
  effectively dead.
- **1024px:** the full 224px sidebar is already shown (the rail ends at
  900px). Chat is still cramped (about 544px), and the patient "Chat" tab is
  off-screen.
- **1440px:** the best-fitting width today.
- **1920px:** layouts disagree. The dashboard is centred at about 1216px,
  patient pages float in centred `max-w-4xl` cards under a left-aligned
  header, and Settings stretches to 1600px. The chat composer is about
  1400px wide, and user bubbles hug the right edge.
- **Settings length:** at 700px it is 4921px (en) and 5099px (de); at
  1440px it is 3784px. That is about 8 and 4 screens respectively.
- **Dark mode** is consistent, but cards barely separate from the page
  (`#191919` vs `#202020`), and disabled primary buttons have low contrast.

---

## 3. Design principles

These follow from the logo: one thin, uniform-weight red stroke on a quiet
near-white square, geometric, with lots of whitespace. They also follow from
the chat-first decision in PR #484.

1. **One stroke, one signal.** The logo has exactly one coloured element. The
   UI should too: per view, at most one thing carries the brand colour, and it
   is either the current selection or the single primary action, never
   decoration. Everything else is ink on paper.
2. **Lines, not boxes.** The logo is a stroke, not a filled shape. Separate
   with hairlines and whitespace. Avoid filled panels (`bg-surface-hover`
   cards inside cards), filled badges and filled toggles. Prefer outline and
   underline states.
3. **Uniform weight.** The glyph has no thick/thin contrast. Use two type
   weights (regular and semibold), one stroke width for icons (lucide at
   1.5px), and one border width. Hierarchy comes from size, colour and space.
4. **Generous margins, tight content.** The logo's whitespace is around the
   mark, not inside it. Pages get a calm outer margin and a readable measure
   (about 72ch for prose). Lists and records inside stay dense (32px rows).
5. **The conversation is the workbench; records are the reference.** Chat is
   where work happens. Patient pages are what you consult and correct. Every
   record view offers a direct way to "ask about this in chat", and chat
   always shows which record it is grounded in.
6. **Red means the brand or danger, and the two never touch.** Red is the
   clinical convention for errors (NN/g; NISTIR 7804 draft checklist), so the
   brand red never fills an action button. Danger always carries an icon and
   a verb, never colour alone.
7. **Calm is a safety property.** No motion beyond 150ms colour and opacity.
   No celebratory states. Destructive and irreversible steps (factory reset,
   mnemonic confirmation, export) keep their friction.

---

## 4. Proposed IA and navigation

Recommended options: D2-B and D3-B in §9.

```
┌ Sidebar (source list) ──────┐┌ Content ─────────────────────────────────────┐
│ [ram glyph] RamDoc          ││ Page shell: header (title · context · 1 primary)
│ ─────────────────────────── ││                                               │
│ ✎ New chat          ⌘N      ││                                               │
│ ◦ Chat (home)               ││                                               │
│ ◦ Patients                  ││                                               │
│ ◦ Calendar                  ││                                               │
│ ─────────────────────────── ││                                               │
│ RECENT CHATS (on /chat)     ││                                               │
│   Müller – Verlaufsbericht  ││                                               │
│   …                         ││                                               │
│ ─────────────────────────── ││                                               │
│ ◦ Reference                 ││                                               │
│ ◦ Settings                  ││                                               │
│ 🔒 Lock          [● Model]   ││                                               │
└─────────────────────────────┘└───────────────────────────────────────────────┘
```

- **Sidebar.** Chat is first and is the home. Patients and Calendar follow.
  Reference (renamed from Literature) and Settings move to a secondary group
  at the bottom. Dashboard leaves the nav and its content becomes the chat
  home (D3).
- **Recent chats move into the app sidebar.** This deletes the 256px
  `/chat` list column; it is Apple's source-list pattern (HIG Sidebars: at
  most two levels). The list shows only on `/chat`. On other routes the group
  collapses to "Recent chats ›".
- **Active state is prefix-matched.** `/patients/*` highlights Patients.
- **The TopBar is removed as a permanent row.** Search moves into the
  CommandPalette, opened by Cmd+K or a sidebar "Search" row. Model status
  moves to the sidebar footer as a labelled chip ("Model ready" or "Load
  model") with a ≥24px target. Page-level actions live in the page header.
  On small widths, a 44px header row carries the menu toggle only when the
  sidebar is overlaid.
- **Responsive sidebar.** At 1024px and above it is the full 232px. From 700
  to 1023px it is a 52px icon rail with tooltips, and recent chats open as a
  popover. The overlay drawer is kept only for the impossible <700 case, as a
  safeguard.
- **Patient workspace tabs (D4).** Overview · Sessions · Diagnoses ·
  Medications · Treatment plan · Documents. Documents is one list of files,
  reports, letters and emails, with a type filter; the existing routes stay
  and are reached from it. The Chat tab is replaced by an **"Ask in chat"**
  button in the patient header, which opens `/chat?scope=patient&patientId=…`
  with the patient pre-attached, as PR #484 already routes. Existing URLs keep
  working: `/patients/[id]/email` and `/reports` stay as filtered views under
  Documents.

---

## 5. Per-screen redesign

### Shell (`+layout.svelte`, `Sidebar`, `TopBar`, `CommandPalette`, `Toast`)
- Implement §4.
- The sidebar header shows the ram glyph as an inline SVG `BrandMark`
  primitive in `currentColor`, coloured by `--brand`, next to the wordmark in
  `text-heading`. When collapsed, the glyph alone replaces "R".
- Add a skip-to-content link (usability §13).
- Fix snippet highlighting once, in a shared `HighlightedSnippet`
  component. It should parse `<mark>` into text segments and never inject
  HTML. Both current renderers are broken (finding 1b).
- CommandPalette: add a "Search records" group fed by `globalSearch`, the
  logic now in TopBar. Show dates on session and report results
  (usability §2.3). Use the shared scrim token. Keep `max-w-2xl`, but use
  `pt-[12vh]` instead of `pt-32` so it fits at 600px height.

### Chat home (`/chat`, `ChatThread`, `ChatMessage`, `ChatSessionList`, `context-picker/*`)
- **Thread column.** Center the conversation in a `max-w-[72ch]`-ish measure
  (a `--measure-chat` token, about 760px) at every width. Composer and
  messages share that column.
- **Markdown.** Render assistant markdown, using the sanitized renderer
  already used for release notes in Settings (`renderMarkdown`) or an
  equivalent shared util. This is P0 and is the most visible defect in the
  primary workspace.
- **Context and selection honesty.** Don't auto-select the newest chat on
  load; show the Today home instead. A patient-scoped chat must show its
  patient in the context bar, not "No patient context attached".
- **Messages.** User messages become right-aligned, `bg-surface-sunken`,
  hairline-bordered and ink text, not accent fill. Assistant messages are
  unboxed prose (Notion and Claude style), with provenance and draft review
  as hairline cards underneath. This removes the largest accent surface in
  the app.
- **Context.** Context moves from an inline block to a **context bar** above
  the composer: chips for attached patients and sources, plus "Attach…". The
  bar opens the existing `ContextPicker` in a popover at ≥1024px and a
  `Dialog` below that. The thread never gets pushed down. The
  "context will be attached" accent banner becomes a muted caption in the
  bar.
- **Empty state (no chat selected / new chat).** A "Today" summary (D3):
  today's sessions, incomplete notes and recent patients, as compact lists.
  Each row offers an action such as "Write note for 10:00 – Müller" or "Ask
  about Meier", which starts a chat with that context pre-attached. Also
  show 3–4 neutral suggested prompts (usability §9). The data comes from the
  existing `getDashboardData`.
- **Composer.** One primary: Send, as an ink icon button, with Enter to send
  and Shift+Enter for a newline. The model name and effort select become a
  quiet footer row. When the model isn't loaded, the composer shows an inline
  "Load model" action instead of a disabled textarea with a settings-pointing
  placeholder (usability §11).
- **Chat list (now in the sidebar).** Rename and delete move to a `⋯`
  `Menu` primitive per row, which is keyboard reachable. Deletion uses
  `ConfirmDialog`.

### Patients list (`/patients`, `PatientCard`)
- Use `PageShell` with the title, search and "New patient" (primary) in the
  header. Show a dense list (36px rows) with columns for name, DOB, AHV and
  last seen; columns collapse at 700px. Add "last seen" (usability §3).

### Patient workspace (`patients/[id]/+layout.svelte`, overview)
- **Header.** A persistent identification strip: name (`text-title`), DOB
  formatted, age, AHV in `data-numeric`. Actions on the right: **Ask in
  chat** (primary), New session, and a `⋯` menu with Edit, Export FHIR,
  Export PDF, Send email, and Delete (danger, separated by a divider, opens
  `ConfirmDialog`).
- **Tabs.** Six underline tabs (§4), with the active tab indicated by an ink
  2px underline. No horizontal scroll at 700px: overflow tabs collapse into
  "More ▾".
- **Subpages drop their own `<h1>` and outer `p-8`.** They render a section
  toolbar (section title as `text-heading`, filters, one action) inside the
  shared shell.
- **Header height.** The header, tabs and section toolbar together must
  take at most about 120px at 700×600. Today they take about 200px.
- **Width.** Every tab uses the same content width and left edge as the
  header (`PageShell width="wide"`). Text-heavy editors center inside it at
  the prose measure.
- **Overview.** `PatientHistoryQuery`'s separate composer is replaced by a
  compact "Ask about this patient" input. On submit it opens `/chat` with
  the patient attached and the question pre-filled, so there is one chat
  implementation instead of two. Its suggested queries become the chat
  home's patient-scoped starters. This needs PO confirmation, because
  PatientHistoryQuery has its own evidence and audit pipeline
  (`query_patient_history`). If that audit UI must stay distinct, keep the
  panel and restyle it as a clearly labelled "Evidence query", not a chat.
- **Overview layout** becomes a two-column summary at ≥1280px: on the
  left, the ask input and a recent-activity timeline; on the right, a key facts
  card (active diagnoses, current medications, latest scores, contact and GP)
  as a definition list. At 1024px and below it is one column. Edit opens the
  existing `PatientForm` in place. The trend charts stay collapsible.

### Sessions, diagnoses, medications, treatment plans
- Use the shared list-row primitive, `Button`, `ConfirmDialog` instead of
  `confirm()`, and lucide chevrons instead of inline SVGs. Allow one primary
  per view: "New session", "Add diagnosis", and so on.
- Treatment plans (958 lines): split it into `TreatmentPlanCard`,
  `GoalList` and `InterventionList` components. Its 7 accent buttons become
  one primary plus secondary and ghost buttons.
- Session detail: the note editor uses the prose measure. "Generate summary"
  and "Draft report" become secondary buttons, and the latter links into
  chat with the session attached, in line with PR #484.

### Documents (files, reports, letters, email)
- Add a new `Documents` tab view: a unified list with a type filter
  (`SegmentedControl`). It reuses the existing list APIs (`list_files`,
  `list_reports`, `list_letters`, `list_emails`); no backend change.
- The report, letter and email editors use the prose measure, with a sticky
  bottom action bar (Save as secondary, Finalize/Send as primary). The green
  "success" buttons (`reports/[rid]:182`, `letters/new:379`,
  `sessions/[sid]:470`) become normal primaries. Success is a state, not an
  action colour.

### Calendar
- The week/month toggle becomes a `SegmentedControl` (ink selection, not an
  accent fill). Prev, next and today become `IconButton`s with aria-labels;
  the current "‹ ›" glyphs have no labels.
- Add a weekday and date header row, sticky while scrolling, and a today
  column tint plus a "now" line. Event height should be proportional to
  duration. Status gets a text label or a legend (usability §13).
- Use a locale-aware date formatter from a shared util (`$lib/format.ts`)
  instead of hardcoded `de-CH`.
- Below 1024px the week view switches to a 5-day work week, or an agenda
  list at 700px.

### Reference (renamed from Literature)
- Two sections with a `SegmentedControl` or sub-tabs: **Medication
  reference** (search-first) and **Literature library** (upload and search).
  The decorative accent icon chip goes away (`literature:239`).

### Settings (rebuilt; D6)
- Split into sub-routes with a left sub-nav inside the page shell. The
  sub-nav becomes a `Select` at 700px.
  - **General:** language, appearance (a `SegmentedControl`, not
    radio cards).
  - **Practice:** new. It edits `PracticeSettings` (name, address, ZSR,
    canton, therapist, specialty) and reuses the onboarding step1 form. This
    closes the "editable only once" gap.
  - **Models:** writing model, router, inference profile, installed,
    available and recommended models. Expert diagnostics (KV cache, batch,
    residency, benchmark) go behind a "Show diagnostics" disclosure.
  - **Reference data:** embedding model, medication reference.
  - **Import & backup:** CSV import and backup/restore.
  - **Updates & about:** version (shown once), check/install, release notes.
  - **Danger zone:** emergency export and factory reset. The typed-word
    confirmations stay exactly as they are.
- Each row uses a `SettingsRow` primitive: label and description on the
  left, the control right-aligned with `shrink-0`, stacking below 640px of
  content width. This removes the overflow class of bugs.
- Status lines get text labels, for example "Loaded · nomic-embed-text-v1.5",
  not just a coloured dot. Badges use `Badge` (subtle tone, outlined).
- Content measure is `max-w-3xl` for forms and `max-w-5xl` for model lists.

### Auth and onboarding
- **Visual only.** A centered card on `bg-surface-sunken` with the ram glyph
  above the title. Use the `Button` primitive (32px, not `px-6 py-3`), the
  `Field` and `Input` primitives, and a step indicator for onboarding.
- **Not changed:** the mnemonic display and the word confirmation logic, the
  `RESET` typed confirmation, recovery routing, and the Touch ID and unlock
  error handling. See §7.

---

## 6. Tokens and primitives

Everything stays inside the existing token model: `:root` plus `.dark`,
`@theme inline`, no `dark:` colour variants, no raw palette. `check:design`
and `check:i18n` must keep passing, and both are extended as described below.

### 6.1 Colour (assumes D1 = B, "ink + red signal")
- New `--brand: #dc0000`, sampled from the icon (the PNG's dominant red).
  Dark mode uses about `#ff4d4d`, to be tuned to ≥4.5:1 on `--surface`. It
  also gets `--brand-subtle`. `--brand` is used **only** for the brand mark,
  the active-navigation indicator bar, and the selected tab underline if
  approved. It is never used for buttons or text.
- `--accent` is repurposed as **ink**: `#1d1d1b` in light and `#ededec` in
  dark, with `--on-accent` inverted. The primary button becomes an ink-filled
  button, in the style of Linear, Vercel, Notion's modal primaries, and
  Apple's default buttons in graphite accent. `--accent-subtle` and
  `--accent-line` become neutral (selected-row tint).
- New `--link` for link text. It can stay a muted blue (`#1d4ed8` and
  `#8fb4fb`) or be ink with an underline (sub-decision in D1).
- New `--focus` for the focus ring, a separate token. It defaults to a
  high-contrast ink-blue ring, so a focused input never looks like an error
  (red) and a focus ring never looks like a selection.
- `--info` currently aliases `--accent`. It must become its own blue so info
  alerts don't turn ink.
- Danger keeps a red hue but moves away from the brand red
  (`#dc2626` → `#c42b1c`-ish, tuned for contrast). It always pairs with an
  icon, and danger buttons are outline-danger at rest and filled only inside
  a `ConfirmDialog`.
- Neutral temperature (D8): optionally shift the surface ramp from warm
  (`#fbfbfa`) toward the icon's cool `#f0f0f5` family, for example a
  surface of `#fafafc` and sunken `#f3f3f6`.
- Add a `--scrim` token, one per theme, replacing all `bg-black/NN`. Extend
  `check:design` so `bg-black/NN` and `bg-white/NN` are rejected and the
  allow-list moves into `app.css`.

### 6.2 Type
- Keep the system stack (SF Pro on macOS). Keep the six steps but tune
  them: `display` 24px (was 26) because the page header no longer competes
  with a TopBar; `title` 18px (was 20); `heading` 15px; and `body`, `label`
  and `caption` unchanged.
- Add `--measure-prose: 72ch` and `--measure-chat: 760px` (as `max-w-prose`
  and `max-w-chat` utilities).
- Extend `check:design` to reject `text-(xs|sm|base|md|lg|xl|[2-9]xl)`,
  which would have caught `text-md`. Also reject `font-bold`.

### 6.3 Space, size and geometry
- Control heights (D5): restore the documented **32px default and 28px
  dense** in `Button`, `IconButton`, `Input` and `Select`. An `lg` (36px)
  size is allowed only for the chat composer send and auth screens. This
  reverses PR #484's `min-h-10`; see D5 for the reasoning (macOS is not a
  touch platform, and WCAG 2.5.8 requires 24px).
- Page padding tokens: `--page-x: clamp(16px, 3vw, 40px)` and
  `--page-y: 24px`, used only by `PageShell`.
- Radii unchanged (6 and 10). Elevation unchanged.

### 6.4 New and changed primitives (`src/lib/components/ui/`)
| Primitive | Purpose |
| --- | --- |
| `PageShell` | The single page container: header slot (title, description, breadcrumb, actions), `width="prose|form|wide|full"`, padding tokens, scroll ownership. It replaces the 8 ad-hoc recipes. |
| `SectionHeader` | `text-heading` + description + right-aligned actions, for sections inside pages and tabs. |
| `SettingsRow` | Label and description with a right-aligned control (`shrink-0`, `whitespace-nowrap`); it stacks when narrow. |
| `ListRow` | 36px dense row: leading icon, primary and secondary text, meta, trailing actions slot. Used by patients, sessions, documents and chats. |
| `Tabs` | Underline tabs with overflow into "More". Used for the patient workspace, Reference and Settings on narrow widths. |
| `SegmentedControl` | Mutually exclusive options (calendar view, theme, document type). |
| `Menu` | Accessible dropdown (button + `role=menu`, arrow keys, Esc) for `⋯` actions and Export. It replaces the hand-rolled export menu. |
| `ConfirmDialog` | Wraps `Dialog`: title, body naming the record, cancel and destructive confirm. It replaces `window.confirm()` and hand-rolled modals. |
| `StatusIndicator` | Dot plus text label; tones ok, warn, error, idle. |
| `ProgressBar` | Replaces the 4 hand-rolled bars in Settings. |
| `BrandMark` | Inline SVG of the ram glyph in `currentColor`, sized 16/20/32/64. |
| `Tooltip` | For rail icons and icon buttons; CSS only, no dependency. |
| `Button` | Heights per D5. Add `size="lg"` and `variant="danger-outline"`. Keep `href` handling. |
| `Badge` | Subtle tones only; remove filled usage. |

`docs/design-language.md` gets updated in the foundation workstream: the
accent and brand rule, the measure tokens, the new primitives, the height
rule, and the scrim.

---

## 7. Risks and what not to change

**Do not change:**
- Clinical workflows and data: AMDP capture, session note semantics, report
  draft provenance and claim gating (PR #484), the review-gated save, the
  `chat_draft_versions` behaviour, and context-plan serialization
  (`context-picker/state.ts`). The redesign moves where the picker renders,
  not what it produces.
- Persisted values: localStorage keys for theme, language, report presets
  and thinking effort; `PracticeSettings` field names; route URLs, which can
  be added to but not removed (keep redirects); and the `?scope=patient&patientId=`
  chat deep link.
- Auth safety: the mnemonic display, the confirm-3-words step, the `RESET`
  typed confirmation, the export confirm word, the unlock and recovery error
  routing (`unlock-errors.ts`), and Touch ID flows. Visual restyling only;
  every one of these keeps its current friction.
- Keyboard shortcuts: Cmd+K and Cmd+N. Cmd+N's meaning changes to "New chat"
  only if the PO agrees (D2). Cmd+Shift+S stays.
- The design-language rule that views never name a colour, and the i18n rule
  that views never write text.

**Risks:**
- **Translation files are the merge hotspot.** Every workstream adds keys to
  `de.json` and `en.json`. Mitigation: each workstream adds keys only under
  its own namespace, appending at the namespace end, and rebases on `main`
  before PR. The i18n check catches drift.
- **Accent semantics change.** Every `bg-accent` call site flips from blue to
  ink at once in the foundation PR. That is intended, but every screen should
  be eyeballed in both themes.
- **Height change.** 40→32px buttons alter layouts that PR #484 tuned for
  700px. Verify at 700px.
- **Removing the TopBar** changes the search entry point. Mitigate with a
  visible "Search ⌘K" row in the sidebar.
- **ESLint is not green on `main` (69 errors).** Workstreams must not add
  errors in files they touch, and should fix `svelte/require-each-key` in
  files they rewrite.
- **The mock-based screenshots are not the Tauri WKWebView.** Font rendering
  and native `<select>` look different there. Final verification needs
  `pnpm tauri dev` on macOS.

---

## 8. Implementation workstreams

Order: **WS0 first**; it must merge before the others start. WS1–WS6 then
run in parallel. Every workstream follows AGENTS.md (branch, conventional
commit, PR with a version label; `minor` for WS0–WS2, `patch` otherwise).

**Common acceptance criteria for every workstream:**
- `pnpm check:design`, `pnpm check:i18n`, `pnpm exec svelte-check` (0/0),
  `pnpm test` and `pnpm format:check` pass.
- `pnpm lint` reports no new problems in the files touched.
- Screenshots at 700×600, 1024×768, 1440×900 and 1920×1080, light and dark,
  attached to the PR for every route the workstream owns. Use the mock render
  harness if it is adopted as a dev tool (see the note below), otherwise
  `pnpm tauri dev`.
- No raw `<button>`, `<input>`, `<select>` or `<textarea>` in owned files
  unless documented; no `window.confirm`; at most one `variant="primary"`
  per view.
- No horizontal page scroll (`scrollWidth <= clientWidth`) at 700px, and no
  button with `scrollWidth > clientWidth`.

### WS0: Foundation (tokens, primitives, docs, guards). Blocks all others.
Owns: `src/app.css`, `src/lib/components/ui/*` (new and changed primitives
from §6.4, plus `HighlightedSnippet` and a `Markdown` display component),
a new `src/lib/format.ts` (locale-aware date, time and duration
formatters; one date format app-wide), `src/tests/components/ui.test.ts` (plus new primitive tests),
`docs/design-language.md`, `scripts/check-design-tokens.mjs`,
`scripts/check-i18n.mjs` (detect string literals in `{…}` text
expressions), and the `common.*` translation keys.
Delivers: the D1 colour model, D5 heights, type tuning, measure, scrim and
focus tokens, and the `BrandMark` SVG. Requires the PO-approved vector of
the glyph; also replace `dokassist/app-icon.svg`.
Acceptance: the primitives gallery is rendered in both themes; the new guard
rules fail on a seeded `text-md`, `bg-black/50` and `font-bold`; existing
screens still render with no layout breakage at 700px.

### WS1: App shell and navigation
Owns: `src/routes/+layout.svelte`, `src/routes/+page.svelte`,
`src/lib/components/{Sidebar,TopBar,CommandPalette,Toast}.svelte`, delete
`PatientTabs.svelte`, `ReportTypeSelector.svelte` and
`UpdateNotification.svelte` (dead code), `src/routes/dashboard/+page.svelte`
(redirect to `/chat` or slim per D3), `src/lib/stores/{router,engine}.ts`
read-only, and `nav.*`, `topbar.*`, `commandPalette.*`, `dashboard.*` keys.
Depends on: WS0.
Acceptance: the sidebar ordering and grouping from §4; prefix-matched
active state; search folded into the palette; a labelled ≥24px model status
chip; the rail at 700–1023px; a skip link; keyboard focus order verified.

### WS2: Chat workspace
Owns: `src/routes/chat/+page.svelte`,
`src/lib/components/{ChatThread,ChatMessage,ChatSessionList,ThinkingEffortSelect}.svelte`,
`src/lib/components/context-picker/ContextPicker.svelte` (presentation
only; `state.ts` and `types.ts` stay read-only), a new
`src/lib/components/chat/TodayPanel.svelte`, `chat.*` keys, and the
`routes/patients/[id]/chat` redirect stub. Depends on WS0; coordinates with
WS1 on the sidebar recent-chats slot. WS1 exposes a snippet or prop, and WS2
supplies the list.
Acceptance: the thread is centered in the measure at all widths, with at
least 520px at 700px; the context bar never pushes the thread; the empty
state shows Today and the prompts; no accent-filled user bubbles; the
existing ChatThread, ChatMessage and ContextPicker tests pass, updated only
for markup.

### WS3: Patient workspace core
Owns: `src/routes/patients/+page.svelte`, `patients/new`,
`patients/[id]/+layout.svelte`, `patients/[id]/+page.svelte`,
`patients/[id]/{sessions,diagnoses,medications,treatment-plans}/**`, and
the components `PatientCard`, `PatientForm`, `AhvInput`, `SessionCard`,
`DiagnosisCard`, `IcdSearch`, `Medication*`, `AMDP*`, `OutcomeScore*`,
`PatientHistoryQuery`, `ReportStream`, plus new
`components/treatment-plans/*`. Also the `patients.*`, `sessions.*`,
`diagnoses.*`, `medications.*`, `treatmentPlans.*`, `outcomeScores.*`,
`patientHistory.*` and `amdp.*` keys. Depends on WS0. This is the largest
workstream; split it into WS3a (list, layout, overview) and WS3b (clinical
tabs) if staffing allows.
Acceptance: the patient identification header on every patient route; 6
tabs with no horizontal scroll at 700px; `ConfirmDialog` for all deletes;
treatment plans split under 400 lines per file; the "Ask in chat" deep link
works.

### WS4: Documents
Owns: `patients/[id]/{files,reports,email,letters}/**`, a new
`patients/[id]/documents/+page.svelte`, `FileCard`, `FileUploader`,
`FileViewer`, `ReportEditor`, `EnhancedReportEditor`, `ErrorDisplay`, and
the `files.*`, `reports.*`, `letters.*` and `email.*` keys. Depends on WS0.
The tab entry is added by WS3: WS3 owns the layout, and WS4 hands over the
route path.
Acceptance: the unified documents list with a type filter; editors in the
prose measure with a sticky action bar; no green action buttons; old URLs
still resolve.

### WS5: Settings
Owns: `src/routes/settings/**` (split into `+layout.svelte` plus sub-route
pages), a new `src/lib/components/settings/*` (one component per pane,
extracted from the 2018-line file), `PromotedModelImportDialog.svelte`, and
the `settings.*` keys. It reuses the onboarding step1 practice form by
extracting `components/PracticeForm.svelte`. WS5 owns the new component;
WS6 switches step1 to it.
Depends on: WS0.
Acceptance: no settings file over 400 lines; every row uses `SettingsRow`;
"Check for Updates" (de and en) fits at 700px; Practice details are editable
after onboarding; diagnostics are behind a disclosure; the typed-word
confirmations are unchanged; the version is shown once.

### WS6: Auth, onboarding, calendar, reference
Owns: `src/routes/{setup,unlock,recover,reset}/+page.svelte`,
`src/routes/onboarding/**`, `MnemonicDisplay.svelte`,
`src/routes/calendar/+page.svelte`, `src/routes/literature/+page.svelte`
(the route stays and the label becomes "Reference"; a `/reference` alias is
optional), `MedicationInfoPanel.svelte`, and the `auth.*`, `onboarding.*`,
`calendar.*` and `literature.*` keys.
Depends on: WS0. Uses `PracticeForm` from WS5 if it is merged; otherwise it
leaves step1 as is and a follow-up swaps it.
Acceptance: auth screens show the brand mark and use the primitives; all
safety confirmations behave exactly as before (existing auth tests plus a
manual walk-through of setup, then lock, unlock, recover and reset); the
calendar is usable at 700px (agenda or 5-day); calendar status has text
labels.

**File-overlap matrix.** The only shared files are `de.json` and `en.json`
(namespace-partitioned) and `ui/index.ts` (WS0 only). WS1 and WS2 meet at
the sidebar slot; WS3 and WS4 meet at the tab list. Both hand-offs are
specified above.

**Quick wins (P0 fixes that don't need to wait for the redesign).** These
could go out as small `fix/` PRs straight away, ahead of WS0:
- the updates button (`whitespace-nowrap shrink-0`, and remove the
  `nth-of-type` grid),
- chat markdown rendering,
- search snippet highlighting,
- the calendar weekday header,
- the "Duration (minutes)" unit,
- the hardcoded German ICD placeholder,
- the hardcoded "measurements" string.

**Render harness.** This audit's screenshots came from a Playwright script
that mocks `window.__TAURI_INTERNALS__` with fixture data (kept local in
`.ux-audit/`, not committed; rerun with
`pnpm exec vite dev --port 5199 --strictPort` then
`node .ux-audit/render.mjs`). Shipping it as `dokassist/scripts/render-screens.mjs`
plus fixtures would give every workstream reproducible before/after
screenshots without an unlocked vault. Recommended as a separate small
`chore/` PR before WS1–WS6, but optional.

---

## 9. Product-owner decisions

Ordered by how much they block implementation. The research notes cite
sources; anything I could not verify is marked *(unverified)*.

### D1. Accent colour: what happens to the logo red? (blocks WS0, and therefore everything)
**Question:** should the UI's action colour become the logo's red
(`#DC0000`), stay blue, or become something else?

| Option | Tradeoffs |
| --- | --- |
| A. Red accent everywhere (buttons, links, focus, selection) | Strongest brand tie. But red is the near-universal error and danger colour; today's `--danger` (`#dc2626`) is almost the same hue as the logo (`#dc0000`). Primary buttons would look destructive, and a red focus ring on an input reads as a validation error. Danger would need re-hueing (to magenta or crimson) and still collides. |
| **B. Ink primary + red as a brand signal only** (recommended) | Primary buttons and selection in near-black ink. The red appears only in the brand mark and the 2px active-nav bar: one stroke, as in the logo. Danger stays red but keeps its meaning. Calm and clinical. The cost is less "colour" in the UI; links need their own token. |
| C. Keep blue (status quo) | No work and no risk, but the logo and UI stay unrelated, and blue is the generic SaaS default the redesign is meant to move away from. |
| D. Red accent, danger moved to another hue plus mandatory icons | A compromise that breaks clinical convention for danger, a poor trade in a medical record. |

**Recommendation:** B, high confidence on "not A", medium on B versus C.
Evidence: NN/g advises reserving warm bright colours like red for warnings
and errors, and keeping colour meaning consistent across screens
([NN/g, Visual hierarchy](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/);
[NN/g, Using color](https://www.nngroup.com/articles/color-enhance-design/)).
The NIST EHR usability protocol draft checklist lists red for errors and
yellow for warnings
([NISTIR 7804](https://nvlpubs.nist.gov/nistpubs/ir/2012/NIST.IR.7804.pdf);
the wording is seen in a secondary copy only and was not verified against the
final PDF). Ink-primary is the Linear, Vercel and Notion convention; this is
an observation of those products, not a cited study. Contrast:
`#DC0000` on white is about 5.2:1 (AA for text) and on the icon's `#F0F0F5`
about 4.6:1 (computed). **Sub-decisions:** links as muted blue or as
ink+underline (recommend blue: it is the most recognisable link affordance
and has no clash with the brand); and whether the selected tab underline
uses brand red or ink (recommend ink, so red stays at one use per screen).

### D2. Navigation model (blocks WS1 and WS2)
**Question:** how do chat, records and tools share the window?

| Option | Tradeoffs |
| --- | --- |
| A. Keep the flat 6-item sidebar and reorder it so Chat is first | Minimal change, but the chat page keeps its own second list column (three columns at 700px) and the Dashboard stays a separate home. |
| **B. Source-list sidebar: primary destinations + recent chats in the sidebar; TopBar removed; search in Cmd+K** (recommended) | Matches the macOS source-list pattern and chat-first apps (ChatGPT, Claude, Linear's inbox). It frees about 256px at every width. The cost is a WS1/WS2 hand-off, and search moves to Cmd+K plus a sidebar row. |
| C. Chat-only shell; records open as side panels inside chat | Purest chat-first, but it buries structured records that clinicians must consult and correct, and it is a big rewrite of patient routes. Too far for one redesign. |

**Recommendation:** B, medium-high confidence. Apple's HIG says sidebars
should show at most two hierarchy levels and be hideable but not hidden by
default
([HIG Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)).
B respects both. **Sub-decision:** should Cmd+N mean "New chat" (consistent
with chat-first) or keep "New patient"? I recommend Cmd+N for New chat and
Cmd+Shift+N for New patient, with low confidence: this is a muscle-memory
change for existing users.

### D3. What happens to the Dashboard? (blocks WS1 and WS2)
| Option | Tradeoffs |
| --- | --- |
| A. Keep `/dashboard` as a separate nav item | No change, but there are two homes. |
| **B. Fold it into the chat home as a "Today" panel; `/dashboard` redirects** (recommended) | The morning context becomes one click from action ("write note for 10:00 → chat with session attached"). The cost is a busier empty chat state. |
| C. Delete it | Loses the useful "incomplete notes" QA signal (usability §3 calls it particularly valuable). |

**Recommendation:** B, medium confidence. It directly operationalises PR
#484's "chat is the primary clinical workspace" without losing the panels
the usability study rated well.

### D4. Patient workspace structure (blocks WS3 and WS4)
| Option | Tradeoffs |
| --- | --- |
| A. Keep 9 tabs and fix styling only | Least risk, but the tab strip still scrolls at ≤1024px, and the Chat tab is a redirect. |
| **B. 6 tabs (Overview, Sessions, Diagnoses, Medications, Treatment plan, Documents) + "Ask in chat" in the header** (recommended) | Fits 700px, surfaces Letters, and makes chat the explicit action. The cost is a new Documents aggregate view; old URLs stay. |
| C. Group into 3 (Overview, Clinical, Documents) with sub-tabs | Fewest tabs, but two levels of tabs adds clicks for frequent items (sessions, meds). |
| D. Patient chat docked as a right inspector at ≥1440px | Attractive later. It needs a chat thread that can live outside `/chat`. Deferred. |

**Recommendation:** B now, D as a follow-up. Medium confidence.

### D4a. One chat or two? (blocks WS2 and WS3 at the patient overview)
**Question:** the patient overview's "Ask about this patient"
(`PatientHistoryQuery`, backed by `query_patient_history` with an evidence
manifest and answer audit) is a second conversational UI beside `/chat`.
Should it be merged?
- **A. Merge.** The overview input hands off to `/chat` with the patient
  attached. This needs `/chat` to surface the same evidence and audit
  metadata, or the audit is lost, so it is a capability question, not just
  UI.
- **B. Keep it separate, but rename and restyle it as "Evidence query"**: a
  one-shot, audited, citation-first lookup, visually distinct from
  conversation.
- **C. Status quo.**

**Recommendation:** B now, A once chat answers carry span-level citations,
which PR #484 lists as a known gap. Medium confidence; it depends on how
much the audited path matters clinically, which is the PO's call.

### D5. Density and control height (blocks WS0)
| Option | Tradeoffs |
| --- | --- |
| **A. 32px default / 28px dense, as documented** (recommended) | Matches the documented design language and the density of macOS and Linear. Recovers vertical space at 600px height. It reverses PR #484's 40px bump. |
| B. 36px compromise | Slightly airier; it is a third height to keep consistent. |
| C. Keep 40px (`min-h-10`) | The current primitives. Generous for a desktop app with a mouse or trackpad; mixed with the 32px raw buttons today. |

**Recommendation:** A, medium-high confidence. I am deliberately departing
from PR #484 here. Its rationale was touch targets, but RamDoc ships only on
macOS, where input is pointer-based. WCAG 2.2 SC 2.5.8 requires 24×24 CSS px
targets, which 32px exceeds
([W3C](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html);
the 24px figure is from memory of the spec, not re-fetched). Apple's
regular push button is far smaller than 40px *(unverified exact height)*.
Keep 36px for the composer and auth screens.

### D6. Settings architecture (blocks WS5)
| Option | Tradeoffs |
| --- | --- |
| A. One page with a sticky in-page section nav | Small change, but it is still a 2000-line route and file. |
| **B. Sub-routes with a left sub-nav (General, Practice, Models, Reference data, Import & backup, Updates & about, Danger zone)** (recommended) | Each pane is small and separately testable; deep links (e.g. "Load model" → `/settings/models`) are possible. |
| C. A separate macOS Settings window (Tauri multi-window), toolbar panes | Most native: Apple's HIG settings pattern uses a toolbar of panes
([HIG Settings](https://developer.apple.com/design/human-interface-guidelines/settings)). But it is Tauri multi-window work, it needs auth state shared across windows, and it is hard to test in a browser. |

**Recommendation:** B, high confidence. C is a possible later step; B's pane
components port to it directly.

### D7. Brand mark asset (blocks WS0's `BrandMark`, but can be stubbed)
**Question:** approve an SVG redraw of the ram glyph: a single
uniform-width stroke (two arcs and a stem) traced from
`src-tauri/icons/128x128@2x.png`?
Options: A, the PO supplies the source vector; B, an implementer traces it
(fast, risk of subtle drift); C, ship without an in-app mark (text
wordmark only). **Recommendation:** A if the vector exists, otherwise B with
PO sign-off on a screenshot. Either way, replace the obsolete
`dokassist/app-icon.svg`. Confidence: high that the mark should appear in
the sidebar and on auth screens.

### D8. Neutral temperature (non-blocking; can be tuned in WS0 review)
A: keep the warm Notion-like ramp (`#fbfbfa`). B: shift cool to echo the
icon tile (`#f0f0f5`), e.g. a surface of `#fafafc`. C: pure neutral greys.
**Recommendation:** B, low-medium confidence. The icon tile is cool, and the
red reads crisper on a cool neutral. This is a taste call; review it
side-by-side in the WS0 gallery.

### D9. Literature → "Reference" and its nav weight (non-blocking; WS1 and WS6)
A: keep "Literature" as a primary item. B: rename to "Reference" and move it
to the secondary group, with the medication reference first (recommended).
C: remove it from nav and reach it only via Cmd+K and in-context links (e.g.
from Medications). **Recommendation:** B, medium confidence. The usability
study flagged the page as a dead end in primary nav, and the medication
reference is genuinely useful and should stay one click away.

### D10. Typography (non-blocking)
A: keep the system stack, SF Pro (recommended). It is native, has no
bundling, and keeps the CSP simple. B: bundle Inter (cross-platform
consistency the app doesn't need, since it is macOS only). C: a serif for
long-form clinical prose (reports, letters) only, for example the
system `ui-serif` (New York): distinctive and good for reading. It could be
a later opt-in. **Recommendation:** A, high confidence. Consider C for the
report and letter editors as an experiment.

---

## 10. Appendix: evidence pointers

- Baseline counts were made with ripgrep on `main`. Raw elements: 202
  `<button>`, 54 `<input>`, 15 `<select>`, 26 `<textarea>`. Accent fills: 87
  in 36 files. Primitive imports: Button 7, Dialog 1, IconButton 0, Kbd 0,
  Skeleton 0, PageHeader 4, EmptyState 2.
- Logo colours, sampled from the PNG: red `#DC0000` and tile `#F0F0F5`.
- Rendered screenshots: 213 PNGs in `.ux-audit/screenshots/` in the audit
  worktree, named `<route>__<width>[__dark|__de|__full|__700x600].png`.
  Every route was rendered at 700, 1024, 1440 and 1920px. Dark mode was
  rendered at 1440px for 5 routes and German for 5 routes. They are local,
  ignored via `.ux-audit/.gitignore`, and not committed.
- Rendered vs inferred: rendered findings are marked as such in §2. The
  others come from static review. The mobile drawer state could not be
  rendered, because it can't be reached at ≥640px. Native controls and
  fonts differ in WKWebView, so final verification needs `pnpm tauri dev`.

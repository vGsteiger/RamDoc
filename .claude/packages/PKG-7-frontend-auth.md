## PKG-7 — Frontend Shell + Auth Flow

**Goal**: App chrome (sidebar, top bar, routing) and the authentication/onboarding UI.

**Depends on**: PKG-1 (auth commands)

**Files**:

```
src/
├── routes/
│   ├── +layout.svelte        # Main layout: sidebar + content area
│   ├── +page.svelte           # Redirect to /patients or /unlock
│   ├── unlock/
│   │   └── +page.svelte       # Touch ID prompt screen
│   ├── setup/
│   │   └── +page.svelte       # First-run: show 24 words, confirm
│   └── recover/
│       └── +page.svelte       # Recovery: enter 24 words
├── lib/
│   ├── stores/
│   │   └── auth.ts            # Auth state store (reactive)
│   ├── components/
│   │   ├── Sidebar.svelte     # Navigation sidebar
│   │   ├── TopBar.svelte      # Search bar + LLM status indicator
│   │   └── MnemonicDisplay.svelte   # 24-word grid display
│   └── api.ts                 # Typed Tauri invoke wrappers for auth
```

**Auth flow (frontend)**:

```
App opens
    │
    ▼
check_auth() ──▶ "first_run"  ──▶ /setup  (generate keys, show 24 words)
    │                                        │
    │                                        ▼
    │                               confirm 3 random words
    │                                        │
    │                                        ▼
    ├──────────────────────────── /patients (main app)
    │                                        ▲
    ▼                                        │
"locked" ──▶ /unlock (Touch ID prompt) ──────┘
    │
    ▼
"recovery" ──▶ /recover (enter 24 words) ────┘
```

**Sidebar navigation items**:

- Patients (main list)
- Calendar
- Settings
- Lock button (bottom)

**Acceptance criteria**:

- [ ] First launch shows setup flow with 24-word mnemonic grid
- [ ] Mnemonic confirmation requires re-entering 3 random words correctly
- [ ] Subsequent launches show Touch ID prompt
- [ ] Successful auth navigates to patient list
- [ ] Lock button zeroes keys and returns to unlock screen
- [ ] Sidebar highlights current route
- [ ] Cmd+K focuses search bar from any page
- [ ] LLM status indicator: green dot (model loaded) / red dot (no model)
- [ ] Responsive layout: sidebar collapses at narrow widths

**Effort**: ~14h

-----
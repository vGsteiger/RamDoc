## PKG-11 — Frontend: Report Generation + PDF Export

**Goal**: LLM-powered report generation with streaming preview, editing, and PDF export.

**Depends on**: PKG-4, PKG-2

**Files**:

```
src/routes/patients/[id]/
├── reports/
│   ├── +page.svelte           # Report list
│   └── new/
│       └── +page.svelte       # Report generator
src/lib/components/
├── ReportEditor.svelte         # Markdown editor with preview
├── ReportTypeSelector.svelte   # Befundbericht | Verlauf | Überweisung
└── ReportStream.svelte         # Streaming LLM output display
```

**Report generation flow**:

```
Doctor selects report type
    │
    ▼
Selects sessions/date range to include
    │
    ▼
Click "Generate" → invoke('generate_report', { ... })
    │
    ▼
Backend: assemble context → build prompt → stream from embedded LLM
    │
    ▼
Frontend: listen('report-chunk') → append to editor in real-time
    │
    ▼
Doctor edits text → clicks "Finalize"
    │
    ▼
Backend: save to reports table → generate PDF
    │
    ▼
PDF viewable / exportable
```

**PDF generation**: Rust-side using a lightweight HTML-to-PDF approach or `printpdf` crate.
Alternative: use Tauri's `webview.print()` for macOS-native PDF export from rendered HTML.

**Acceptance criteria**:

- [ ] Report type selection with context summary (what data will be included)
- [ ] LLM streaming displays text appearing in real-time
- [ ] Doctor can edit generated text before finalizing
- [ ] Finalized report saved to DB with model name and prompt hash
- [ ] PDF export with proper German formatting and practice letterhead
- [ ] Report list shows all previous reports, filterable by type
- [ ] "Regenerate" button re-runs LLM without losing the original
- [ ] Graceful handling when no model is downloaded yet (shows setup prompt)

**Effort**: ~14h

-----
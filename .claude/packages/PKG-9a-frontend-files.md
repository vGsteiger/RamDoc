## PKG-9a — Frontend: File Browser + Upload

**Goal**: File browser tab on patient detail, drag-and-drop upload with LLM indexing.

**Depends on**: PKG-3, PKG-4, PKG-2

**Files**:

```
src/routes/patients/[id]/
├── files/
│   └── +page.svelte
src/lib/components/
├── FileUploader.svelte        # Drag-and-drop zone
├── FileCard.svelte            # File list item with metadata tags
└── FileViewer.svelte          # In-app PDF/image viewer (decrypted in memory)
```

**Upload flow**:

```
User drops file ──▶ Frontend reads bytes
    │
    ▼
invoke('upload_file', { patientId, filename, data, mimeType })
    │
    ▼
Backend: encrypt → store in vault → create DB record → return FileRecord
    │
    ▼
async: extract text (PDF/OCR) → send to embedded LLM → parse metadata → update DB → index in FTS5
    │
    ▼
Frontend: file card updates with tags, summary, document type (reactive via polling or event)
```

**File viewer**: Decrypt in memory, display in-app. PDFs via `<iframe>` or `<embed>` with blob URL. Images via `<img>` with blob URL. Blob URLs revoked after viewing.

**Acceptance criteria**:

- [ ] Drag-and-drop upload works for PDF, PNG, JPG, DOCX
- [ ] File appears in list immediately after upload (before LLM extraction)
- [ ] LLM metadata populates asynchronously (tags, summary, document type)
- [ ] Click file → decrypted view in-app (no temp file written to disk)
- [ ] Download button exports decrypted file via native save dialog
- [ ] Delete file removes from vault + DB + search index
- [ ] Files sorted by upload date, filterable by document type tag
- [ ] Upload progress indicator for large files

**Effort**: ~14h

-----
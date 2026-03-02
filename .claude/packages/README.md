# DokAssist — Implementation Packages

> Each package is a self-contained unit of work with explicit interface contracts,
> dependencies, acceptance criteria, and test specifications. Packages can be
> assigned to sub-agents in dependency order.

-----

## Overview

This directory contains detailed specifications for all implementation packages in the DokAssist project. Each package is documented in its own file for better organization and maintainability.

## Dependency Graph

```
PKG-0 (Scaffold)
  ├──▶ PKG-1 (Crypto Core + Keychain)
  │      ├──▶ PKG-2 (Database / SQLCipher)
  │      ├──▶ PKG-3 (Encrypted Filesystem)
  │      └──▶ PKG-9 (Backup & Recovery)
  ├──▶ PKG-4 (LLM Client)
  ├──▶ PKG-5 (Search Engine) ◀── PKG-2
  ├──▶ PKG-6 (Audit Logger) ◀── PKG-2
  └──▶ PKG-7 (Frontend Shell + Auth) ◀── PKG-1
         ├──▶ PKG-8  (Frontend: Patients)      ◀── PKG-2, PKG-5
         ├──▶ PKG-9a (Frontend: Files)          ◀── PKG-3, PKG-4
         ├──▶ PKG-10 (Frontend: Clinical)       ◀── PKG-2
         ├──▶ PKG-11 (Frontend: Reports + PDF)  ◀── PKG-4, PKG-2
         └──▶ PKG-12 (Build & Distribution)     ◀── ALL
```

**Critical path**: PKG-0 → PKG-1 → PKG-2 → PKG-5 → PKG-8 (patient list with search is the first usable feature)

**Parallelizable after PKG-1**: PKG-2, PKG-3, PKG-4 can all be built concurrently.

## Package Files

- [PKG-0: Project Scaffold](./PKG-0-scaffold.md) — Empty Tauri 2 + Svelte 5 app (3h)
- [PKG-1: Crypto Core + Keychain + Touch ID](./PKG-1-crypto.md) — Security foundation (14h)
- [PKG-2: Database (SQLCipher)](./PKG-2-database.md) — Encrypted SQLite with migrations (16h)
- [PKG-3: Encrypted Filesystem](./PKG-3-filesystem.md) — File vault operations (10h)
- [PKG-4: LLM Engine (Embedded)](./PKG-4-llm.md) — Embedded inference, no external dependencies (16h)
- [PKG-5: Search Engine (FTS5)](./PKG-5-search.md) — Full-text search across all entities (8h)
- [PKG-6: Audit Logger](./PKG-6-audit.md) — Append-only audit log for nDSG compliance (4h)
- [PKG-7: Frontend Shell + Auth](./PKG-7-frontend-auth.md) — App chrome and authentication UI (14h)
- [PKG-8: Frontend: Patient Management](./PKG-8-frontend-patients.md) — Patient list, forms, detail views (12h)
- [PKG-9: Backup & Recovery](./PKG-9-backup.md) — Built-in backup tooling (8h)
- [PKG-9a: Frontend: File Browser + Upload](./PKG-9a-frontend-files.md) — File management with LLM indexing (14h)
- [PKG-10: Frontend: Clinical](./PKG-10-frontend-clinical.md) — AMDP, sessions, diagnoses, medications (18h)
- [PKG-11: Frontend: Reports + PDF](./PKG-11-frontend-reports.md) — LLM-powered report generation (14h)
- [PKG-12: Build & Distribution](./PKG-12-build.md) — Production build and hardening (8h)

## Summary Statistics

|Package|Name                            |Effort|Dependencies       |Parallelizable With|
|-------|--------------------------------|------|-------------------|-------------------|
|PKG-0  |Scaffold                        |3h    |—                  |—                  |
|PKG-1  |Crypto + Keychain + Touch ID    |14h   |PKG-0              |—                  |
|PKG-2  |Database (SQLCipher)            |16h   |PKG-1              |PKG-3, PKG-4       |
|PKG-3  |Encrypted Filesystem            |10h   |PKG-1              |PKG-2, PKG-4       |
|PKG-4  |LLM Engine (Embedded, No Ollama)|16h   |PKG-0              |PKG-2, PKG-3       |
|PKG-5  |Search Engine (FTS5)            |8h    |PKG-2              |PKG-6              |
|PKG-6  |Audit Logger                    |4h    |PKG-2              |PKG-5              |
|PKG-7  |Frontend: Shell + Auth          |14h   |PKG-1              |—                  |
|PKG-8  |Frontend: Patients              |12h   |PKG-2, PKG-5, PKG-7|PKG-9a, PKG-10     |
|PKG-9  |Backup & Recovery               |8h    |PKG-1, PKG-3       |PKG-8              |
|PKG-9a |Frontend: Files                 |14h   |PKG-3, PKG-4       |PKG-10, PKG-11     |
|PKG-10 |Frontend: Clinical              |18h   |PKG-2, PKG-7       |PKG-9a, PKG-11     |
|PKG-11 |Frontend: Reports + PDF         |14h   |PKG-4, PKG-2       |PKG-10             |
|PKG-12 |Build & Distribution            |8h    |ALL                |—                  |

**Total: ~159h**

**Critical path**: PKG-0 → PKG-1 → PKG-2 → PKG-7 → PKG-8 (first usable patient list: ~59h)

**Maximum parallelism**: After PKG-1, three agents can work simultaneously on PKG-2, PKG-3, PKG-4. After PKG-7, frontend packages PKG-8, PKG-9a, PKG-10, PKG-11 can be partially parallelized.

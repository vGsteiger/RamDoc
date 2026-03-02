## PKG-12 — Build, Distribution & Hardening

**Goal**: Production build, `.dmg` packaging, Gatekeeper signing, and final security hardening.

**Depends on**: ALL packages

**Tasks**:

- [ ] Tauri build config: CSP headers, disable devtools in production
- [ ] App Sandbox entitlements (if distributing outside App Store)
- [ ] Code signing with Apple Developer ID (optional but recommended)
- [ ] Notarization for Gatekeeper (optional)
- [ ] `.dmg` background image and install UX
- [ ] Hardcoded CSP: no remote scripts, no eval, no inline styles
- [ ] Memory zeroization audit: ensure all key material uses `zeroize`
- [ ] Disable Tauri remote URL navigation
- [ ] Penetration test: attempt to extract keys from running process
- [ ] Write user manual (2-page PDF for the doctor)

**`tauri.conf.json` security settings**:

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'",
      "dangerousDisableAssetCspModification": false
    }
  }
}
```

**Acceptance criteria**:

- [ ] `pnpm tauri build` produces working `.dmg` < 30 MB (excluding bundled model)
- [ ] App launches on clean macOS 14+ without any pre-installed dependencies
- [ ] No remote network calls after initial model download (verified with Little Snitch or `nettop`)
- [ ] DevTools inaccessible in production build
- [ ] User manual covers: install, first run, daily use, backup, recovery

**Effort**: ~8h

-----

## Summary

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
# Security Policy

## Supported Versions

Only the latest release is actively supported with security patches.

| Version | Supported |
|---------|-----------|
| Latest release | ✅ |
| Older releases | ❌ |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Use GitHub's private vulnerability reporting:
**[Report a vulnerability](https://github.com/vGsteiger/RamDoc/security/advisories/new)**

Include in your report:
- DokAssist version (visible in the app's About screen)
- macOS version
- Step-by-step reproduction instructions
- Expected vs. actual behaviour
- Your assessment of the impact / severity

## Response Timeline

| Milestone | Target |
|-----------|--------|
| Acknowledgement | Within 48 hours |
| Triage & severity assessment | Within 7 days |
| Patch released | Within 90 days |

We follow coordinated disclosure. We will credit reporters in the release notes unless you prefer to remain anonymous.

## Scope

In scope:
- Authentication bypass or key material exposure
- Encrypted vault / database compromise
- Local privilege escalation
- LLM prompt injection with data exfiltration
- Supply-chain vulnerabilities in dependencies

Out of scope:
- Issues requiring physical access to an already-unlocked device
- Speculative / theoretical attacks with no proof-of-concept

## Security Architecture

For an overview of the cryptographic design, key storage, and threat model, see the internal security architecture document:
[`dokassist/src-tauri/SECURITY.md`](dokassist/src-tauri/SECURITY.md)

## Audit-log Integrity

Every audit row has an HMAC-SHA-256 over its canonical, length-prefixed contents and
the preceding row's MAC. The signing key is domain-separated from both master keys,
so neither the SQLCipher nor filesystem key alone can derive it. The latest head is stored
outside the database in two alternating, device-bound macOS Keychain items.

On database open, RamDoc verifies the complete chain and requires the trusted
Keychain checkpoint to occur in it. This detects modified, inserted, reordered, or
middle-deleted rows as well as truncation or replacement with an older database.
Audit reads verify the complete chain again. SQLite triggers reject structurally
invalid inserts plus all updates and deletions, while the HMAC function is restricted
to direct application statements so attacker-supplied triggers cannot use the key.

### Residual limitation

No fully local audit system is literally tamper-proof against an attacker who controls
the running application and all of its secrets. Such an attacker can use both the
audit MAC key and Keychain access to forge a new history and checkpoint. The design
instead separates the database and audit-integrity trust boundaries, protecting
against possession of the database/key alone and against offline database tampering.

Mnemonic recovery and an explicitly authorized backup restore can establish a new
checkpoint after the remaining chain verifies. That action necessarily establishes a
new truncation-detection baseline and must remain an explicit user-authorized flow.

SQLite and Keychain cannot participate in one atomic transaction. RamDoc checkpoints
immediately after a committed database operation, but forced process termination in
the interval between those two writes can leave the newest valid suffix unanchored.
That suffix is verified and anchored on the next open; deletion within that narrow
crash interval cannot be distinguished from a transaction that never committed.

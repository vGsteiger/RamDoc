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

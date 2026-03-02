# IbexDoc

[![Rust CI](https://github.com/vGsteiger/IbexDoc/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/vGsteiger/IbexDoc/actions/workflows/rust-ci.yml)
[![Frontend CI](https://github.com/vGsteiger/IbexDoc/actions/workflows/frontend-ci.yml/badge.svg)](https://github.com/vGsteiger/IbexDoc/actions/workflows/frontend-ci.yml)
[![Security Audit](https://github.com/vGsteiger/IbexDoc/actions/workflows/security.yml/badge.svg)](https://github.com/vGsteiger/IbexDoc/actions/workflows/security.yml)
[![Lint & Format](https://github.com/vGsteiger/IbexDoc/actions/workflows/lint.yml/badge.svg)](https://github.com/vGsteiger/IbexDoc/actions/workflows/lint.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

100% local, encrypted macOS application for Swiss patient record management.

## Features

- 🔐 **End-to-End Encryption** - AES-256-GCM encryption with BIP-39 recovery
- 🏥 **Swiss Healthcare Standards** - AHV number validation and Swiss-specific workflows
- 💻 **100% Local** - All data stays on your machine, no cloud dependencies
- 🔍 **Full-Text Search** - Encrypted FTS5 search with SQLCipher
- 🤖 **LLM Integration** - Local AI assistance with Metal GPU acceleration
- 📋 **Audit Logging** - Comprehensive compliance-ready audit trails
- 🛡️ **Security First** - Keychain integration, secure memory handling, PHI protection

## CI/CD Pipeline

This project uses comprehensive GitHub Actions workflows for quality assurance:

- **Rust CI** - Unit tests, integration tests, cross-platform builds
- **Frontend CI** - Svelte/TypeScript build verification
- **Security** - Daily vulnerability scans, dependency audits
- **Coverage** - Automated code coverage tracking
- **Lint & Format** - Code quality enforcement
- **Tauri Build** - Cross-platform app packaging (macOS, Linux)

See [CI/CD Documentation](.github/CI_CD_DOCUMENTATION.md) for details.

## Development

### Prerequisites

- Rust 1.82+ (MSRV)
- Node.js 20+
- pnpm 8+
- macOS (for full feature set) or Linux (limited platform support)

### Build

```bash
# Install dependencies
cd dokassist
pnpm install

# Development mode
pnpm tauri dev

# Production build
pnpm tauri build
```

### Testing

```bash
# Rust tests
cd dokassist/src-tauri
cargo test --lib

# Frontend build test
cd dokassist
pnpm build
```

## Architecture

Built with:
- **Backend**: Rust (Tauri 2, SQLCipher, llama.cpp)
- **Frontend**: Svelte 5 + SvelteKit 2 + Tailwind CSS 4
- **Security**: AES-256-GCM, Argon2, macOS Keychain
- **Database**: SQLCipher with encrypted FTS5 search

## License

MIT License - see [LICENSE](LICENSE) for details.

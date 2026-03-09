# CI/CD Pipeline Documentation

This document describes the comprehensive CI/CD pipeline setup for the RamDoc project.

## Overview

The project uses GitHub Actions for continuous integration and deployment with multiple specialized workflows to ensure code quality, security, and reliability.

## Workflows

### 1. Rust CI (`rust-ci.yml`)

**Triggers:** Push to main/develop/claude branches, PRs to main/develop

**Jobs:**
- **Test Suite** - Runs on macOS with stable and MSRV (1.88.0)
  - Unit tests (`cargo test --lib`)
  - Example runs
- **Build Check** - Verifies release builds work on macOS
- **Clippy** - Runs linter with strict warnings
- **Rustfmt** - Checks code formatting

**Key Features:**
- Matrix testing across Rust versions (stable and MSRV 1.88.0)
- Comprehensive caching for faster builds
- Example tests validated
- macOS-specific testing environment

### 2. Frontend CI (`frontend-ci.yml`)

**Triggers:** Push to main/develop/claude branches, PRs to main/develop

**Jobs:**
- **Frontend Build** - Builds the Svelte/TypeScript frontend
- **Type Check** - Type checking for TypeScript and Svelte components

**Key Features:**
- pnpm dependency management
- Build artifact verification
- Caching for node_modules via pnpm store

### 3. Security Audit (`security.yml`)

**Triggers:** Push to main/develop, PRs, daily cron at 00:00 UTC

**Jobs:**
- **Cargo Audit** - Checks for known vulnerabilities in dependencies
- **Cargo Deny** - License and dependency compliance checking
- **Dependency Review** - GitHub-native dependency scanning (PR only)
- **Trivy Security Scan** - Container and filesystem vulnerability scanning

**Key Features:**
- Automated daily security scans
- SARIF report upload to GitHub Security tab
- License compliance checking (MIT, Apache-2.0, BSD, etc.)
- Advisory database checks against RustSec

### 4. Code Coverage (`coverage.yml`)

**Triggers:** Push to main/develop, PRs to main/develop

**Jobs:**
- **Coverage** - Generates LLVM-based coverage with cargo-llvm-cov
- **Test Coverage Report** - Alternative coverage with Tarpaulin

**Key Features:**
- LCOV format for Codecov integration
- Coverage artifacts uploaded for 30 days
- XML format for compatibility with various tools
- Comprehensive test coverage tracking

### 5. Lint & Format (`lint.yml`)

**Triggers:** Push to main/develop/claude branches, PRs to main/develop

**Jobs:**
- **Rustfmt** - Rust code formatting check
- **Clippy** - Rust linting with custom rules
- **Typos** - Spell checking across codebase

**Key Features:**
- Strict Clippy warnings with allowances for common patterns
- Automated typo detection

### 6. Tauri Build (`tauri-build.yml`)

**Triggers:** Version tags (v*), after successful Release workflow completion

**Jobs:**
- **Tauri Build** - Cross-platform Tauri app builds
  - macOS ARM64 (Apple Silicon)
  - macOS x86_64 (Intel)
  - Linux x86_64 (AppImage + Deb)
- **Release** - Upload artifacts to GitHub release

**Key Features:**
- Multi-platform build matrix
- Artifact uploads (DMG, AppImage, Deb packages)
- Triggered by Release workflow
- Signing support (when keys are configured)

## Caching Strategy

All workflows use intelligent caching:

1. **Cargo Registry & Index** - Dependency metadata
2. **Cargo Build** - Compiled artifacts
3. **pnpm Store** - Node modules
4. **Platform-specific** - Different caches per OS

This reduces build times from ~15 minutes to ~3-5 minutes for cached builds.

## Platform Support

### Linux (Ubuntu 22.04)
- ✅ Build and packaging (AppImage, Deb)
- ✅ Supported for application builds via Tauri

### macOS (14 & Latest)
- ✅ Full test suite including platform-specific features
- ✅ DMG and App bundle creation
- ✅ Metal GPU support for LLM inference
- ✅ Both Intel and Apple Silicon targets
- ✅ Keychain integration

## Required Secrets (Optional)

Configure these in repository settings if needed:

- `CODECOV_TOKEN` - For code coverage uploads to Codecov
- `TAURI_PRIVATE_KEY` - For Tauri app signing
- `TAURI_KEY_PASSWORD` - Password for Tauri private key

## Security Features

1. **Dependency Scanning**
   - Daily automated security audits
   - RustSec advisory database integration
   - License compliance checking

2. **Vulnerability Scanning**
   - Trivy filesystem scanning
   - SARIF reports to GitHub Security
   - Dependency review on PRs

3. **Code Quality**
   - Clippy with strict warnings
   - Format checking (rustfmt)
   - Type checking (svelte-check)

## Test Coverage

The test suite includes:

- **Unit Tests** - Tests across all modules covering:
  - Crypto operations (PKG-1)
  - Database operations (PKG-2)
  - Filesystem encryption (PKG-3)
  - LLM sanitization (PKG-4)
  - Search functionality (PKG-5)
  - Audit logging (PKG-6)

- **Example Tests** - Audit system validation

## Performance Considerations

- **Parallel Testing** - Tests run concurrently where possible
- **Smart Caching** - Aggressive caching strategy
- **Fail-Fast Disabled** - All platform builds complete for visibility
- **Conditional Execution** - Platform-specific steps only run when needed

## Continuous Improvement

The pipelines are designed to be:
- **Extensible** - Easy to add new workflows or jobs
- **Maintainable** - Clear separation of concerns
- **Efficient** - Optimized caching and parallel execution
- **Informative** - Detailed logging and artifact uploads

## Troubleshooting

### Build Failures

1. **macOS: Test failures**
   - Keychain tests require keychain access
   - Some tests may need manual intervention

2. **Frontend: Build failures**
   - Verify Node.js version (20+)
   - Check pnpm-lock.yaml is committed
   - Clear pnpm cache if needed

### Coverage Issues

- Coverage generation may timeout on large codebases
- Use `--timeout 300` for longer-running tests
- Consider splitting coverage into separate jobs

## Future Enhancements

Potential additions:
- [ ] Benchmark tracking over time
- [ ] E2E testing with Playwright/Tauri WebDriver
- [ ] Performance regression detection
- [ ] Automatic changelog generation
- [ ] Deployment to distribution channels (Homebrew, etc.)

## Monitoring

All workflow runs are visible in the GitHub Actions tab. Key metrics to monitor:

- Test success rate
- Build time trends
- Coverage percentage
- Security vulnerability count
- Dependency update frequency

---

**Last Updated:** March 9, 2026
**Maintained By:** RamDoc Team

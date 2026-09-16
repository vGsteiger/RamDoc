# CI/CD Pipeline Documentation

This document describes the comprehensive CI/CD pipeline setup for the RamDoc project.

## Overview

The project uses GitHub Actions for continuous integration and deployment with multiple specialized workflows to ensure code quality, security, and reliability.

## Workflows

### 1. Rust CI (`rust-ci.yml`)

**Triggers:** Push to main/develop/claude branches, PRs to main/develop

**Jobs:**
- **Test Suite** - Single job running on macOS with stable and MSRV (1.95.0)
  - `cargo check --all-targets` - Build check
  - `cargo test --lib` - Unit tests
  - Example run (`cargo run --example test_audit`, continue-on-error)
  - `cargo clippy` - Linting with strict warnings (stable only)
  - `cargo fmt -- --check` - Format checking (stable only)

**Key Features:**
- Matrix testing across Rust versions (stable and MSRV 1.95.0)
- Comprehensive caching for faster builds
- All checks run in a single job for efficiency
- macOS-specific testing environment

### 2. Frontend CI (`frontend-ci.yml`)

**Triggers:** Push to main/develop/claude branches, PRs to main/develop

**Jobs:**
- **Frontend Build & Check** - Builds the Svelte/TypeScript frontend and runs tests
- **Svelte Type Check** - Runs `svelte-check` for TypeScript and Svelte components (continue-on-error if not installed)

**Key Features:**
- pnpm dependency management
- Build artifact verification
- Built-in pnpm store caching in every frontend quality job
- Superseded runs are cancelled when a PR receives a newer commit

### 3. Security Audit (`security.yml`)

**Triggers:** Push to main/develop, PRs, daily cron at 00:00 UTC

**Jobs:**
- **Cargo Audit** - Checks for known vulnerabilities in dependencies
- **Cargo Deny** - License and dependency compliance checking
- **Dependency Review** - GitHub-native dependency scanning (PR only)
- **Trivy Security Scan** - Container and filesystem vulnerability scanning

**Key Features:**
- Automated daily security scans
- Cached audit binaries avoid compiling `cargo-audit` and `cargo-deny` on every run
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
- **Frontend Formatting (Prettier)** - Checks frontend code formatting (continue-on-error if not installed)
- **Frontend Linting (ESLint)** - Lints JavaScript/TypeScript/Svelte code (continue-on-error if not installed)
- **Typo Check** - Spell checking across codebase

**Key Features:**
- Frontend tooling checks are best-effort (continue on error when tools not present)
- Automated typo detection across source files and documentation

### 6. Tauri Build (`tauri-build.yml`)

**Triggers:**
1. Called exactly once by the Release workflow with the newly-created tag
2. Manual dispatch with an explicit existing tag for recovery or rebuilds

**Jobs:**
- **Tauri Build** - Signed macOS ARM64 (Apple Silicon) app and updater bundles
- **Release** - Upload artifacts to GitHub release

**Key Features:**
- Explicit target build matrix
- DMG and signed updater artifact uploads
- A single release trigger prevents duplicate builds and accidental rebuilds of the latest tag
- Signing support (when keys are configured)

## Caching Strategy

All workflows use intelligent caching:

1. **Cargo Registry & Index** - Dependency metadata
2. **Cargo Build** - Compiled test, coverage, and Tauri dependencies keyed by toolchain and lockfile
3. **pnpm Store** - Package downloads resolved through `setup-node` using the lockfile
4. **Platform-specific** - Different caches per OS

This reduces build times from ~15 minutes to ~3-5 minutes for cached builds.

## Platform Support

### macOS (14 & Latest)
- ✅ Full test suite including platform-specific features
- ✅ DMG and App bundle creation
- ✅ Metal GPU support for LLM inference
- ✅ Apple Silicon target
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
   - Verify Node.js version (20.19+)
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

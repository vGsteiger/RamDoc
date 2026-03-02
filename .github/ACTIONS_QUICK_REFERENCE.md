# GitHub Actions Quick Reference

Quick reference for developers working with the CI/CD pipelines.

## Workflow Triggers

### Automatic Triggers

| Workflow | Push (main/develop) | Push (claude/**) | PR | Schedule | Tags |
|----------|---------------------|------------------|-----|----------|------|
| Rust CI | ✅ | ✅ | ✅ | ❌ | ❌ |
| Frontend CI | ✅ | ✅ | ✅ | ❌ | ❌ |
| Security | ✅ | ❌ | ✅ | Daily 00:00 UTC | ❌ |
| Coverage | ✅ | ❌ | ✅ | ❌ | ❌ |
| Lint & Format | ✅ | ✅ | ✅ | ❌ | ❌ |
| Tauri Build | ✅ | ❌ | ✅ | ❌ | ✅ (v*) |

### Manual Trigger

All workflows can be manually triggered from the GitHub Actions tab using "Run workflow" button.

## Local Testing

Before pushing, test locally:

```bash
# Rust tests
cd dokassist/src-tauri
cargo test --lib
cargo clippy --all-targets --all-features
cargo fmt -- --check

# Frontend tests
cd dokassist
pnpm install
pnpm build

# Security audit
cd dokassist/src-tauri
cargo audit
```

## Common Issues

### 1. Linux Build Failures (glib-sys)

**Error:** `The system library 'glib-2.0' required by crate 'glib-sys' was not found`

**Solution:** The CI automatically installs dependencies. Locally on Ubuntu/Debian:
```bash
sudo apt-get install -y libgtk-3-dev libglib2.0-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf
```

### 2. macOS-Specific Test Failures on Linux

**Expected:** Some tests (Keychain, Spotlight) only run on macOS.

**CI Behavior:** These are marked as `continue-on-error: true` on Linux.

### 3. Clippy Warnings

**Configuration:** Clippy is strict with `-D warnings` but allows:
- `clippy::too_many_arguments`
- `clippy::type_complexity`

**Fix:** Address warnings or add appropriate `#[allow()]` attributes.

### 4. Coverage Timeouts

**Issue:** Large test suites may timeout during coverage generation.

**Solution:** CI uses `--timeout 300` (5 minutes per test).

## Workflow Files

| File | Purpose | Key Jobs |
|------|---------|----------|
| `rust-ci.yml` | Core Rust testing | test, build, clippy, rustfmt |
| `frontend-ci.yml` | Frontend validation | frontend-build, svelte-check |
| `security.yml` | Security scanning | cargo-audit, cargo-deny, trivy |
| `coverage.yml` | Code coverage | llvm-cov, tarpaulin |
| `lint.yml` | Code quality | rustfmt, clippy, prettier, eslint |
| `tauri-build.yml` | App packaging | multi-platform builds, release |

## Caching

Workflows use aggressive caching for performance:

- **~/.cargo/registry** - Crate metadata
- **~/.cargo/git** - Git dependencies
- **target/** - Compiled artifacts
- **~/.pnpm-store** - Node modules

To clear cache: Re-run workflow with "Re-run all jobs" (clears per-workflow cache).

## Required Secrets (Optional)

Add in repository Settings > Secrets and variables > Actions:

| Secret | Required For | Purpose |
|--------|--------------|---------|
| `CODECOV_TOKEN` | Coverage | Codecov.io uploads |
| `TAURI_PRIVATE_KEY` | Tauri Build | Code signing |
| `TAURI_KEY_PASSWORD` | Tauri Build | Key password |

**Note:** Workflows continue without these secrets (features degraded).

## Pull Request Checks

When you open a PR, these checks run:

1. ✅ Rust CI (test, clippy, rustfmt)
2. ✅ Frontend CI (build, type check)
3. ✅ Security (audit, deny, dependency review)
4. ✅ Coverage (generate coverage report)
5. ✅ Lint & Format (all linters)

**Merge Requirement:** All required checks must pass (some are allowed to fail).

## Branch Protection

Recommended settings for `main` branch:

- ✅ Require pull request reviews
- ✅ Require status checks to pass:
  - Rust CI / Test Suite
  - Rust CI / Build Check
  - Frontend CI / frontend-build
  - Lint & Format / rustfmt
  - Lint & Format / clippy
- ✅ Require branches to be up to date
- ✅ Require linear history

## Performance

**Typical Run Times** (with cache):

- Rust CI: ~5-8 minutes
- Frontend CI: ~2-3 minutes
- Security: ~3-5 minutes
- Coverage: ~8-12 minutes
- Lint & Format: ~3-5 minutes
- Tauri Build: ~15-25 minutes

**First Run** (no cache): 2-3x longer

## Artifacts

Workflows upload artifacts:

| Workflow | Artifact | Retention |
|----------|----------|-----------|
| Tauri Build | macOS DMG, App | 30 days |
| Tauri Build | Linux AppImage, Deb | 30 days |
| Coverage | Coverage reports (XML) | 30 days |

Download from workflow run page > "Artifacts" section.

## Release Process

To create a release:

1. Create and push a version tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. Tauri Build workflow automatically:
   - Builds for all platforms
   - Creates draft GitHub release
   - Attaches build artifacts

3. Edit release notes and publish

## Monitoring

**GitHub UI:**
- Actions tab > All workflows
- Insights tab > Dependency graph > Dependabot

**Email Notifications:**
- Configure in Settings > Notifications > Actions

**Status Badges:**
- Available in README.md
- Update automatically on each run

## Advanced

### Running Single Job

Can't run single job, but can:
1. Temporarily disable other jobs (comment out in YAML)
2. Create feature branch to test changes
3. Use `act` to run locally (requires Docker)

### Adding New Workflows

1. Create `.github/workflows/new-workflow.yml`
2. Define triggers, jobs, steps
3. Test on feature branch
4. Add status badge to README if public-facing

### Customizing Matrix

Edit `strategy.matrix` in workflow:
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, beta, nightly]
```

### Debugging Workflows

1. Add `continue-on-error: false` to fail fast
2. Add debug step:
   ```yaml
   - name: Debug
     run: |
       echo "Debug info"
       env
       ls -la
   ```
3. Use `act` for local testing:
   ```bash
   act -j test  # Run 'test' job locally
   ```

---

**Questions?** Check [CI/CD Documentation](CI_CD_DOCUMENTATION.md) or open an issue.

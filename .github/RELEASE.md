# Continuous Delivery Pipeline

This repository uses an automated continuous delivery pipeline that creates semantic versioned releases when PRs are merged to the main branch.

## How It Works

### Automatic Version Bumping

When a PR is merged to `main`, the workflow automatically:
1. Determines the version bump type (major, minor, or patch)
2. Creates a new git tag
3. Creates a GitHub release
4. Builds and attaches macOS .dmg and .app artifacts

### Version Bump Rules

The version bump type is determined by (in order of priority):

1. **PR Labels** (highest priority):
   - `major` - Breaking changes (e.g., v1.0.0 → v2.0.0)
   - `minor` - New features (e.g., v1.0.0 → v1.1.0)
   - `patch` - Bug fixes and minor changes (e.g., v1.0.0 → v1.0.1)
   - `skip-release` - Skip creating a release

2. **Conventional Commit Format in PR Title**:
   - `feat!:` or `BREAKING CHANGE` → major bump
   - `feat:` or `feature:` → minor bump
   - `fix:`, `perf:`, `refactor:`, `docs:`, `style:`, `test:`, `chore:` → patch bump

3. **Default**: If no label or conventional commit format is detected, defaults to a **patch** bump

### Examples

#### Using Labels
```
PR Title: "Add user authentication"
Label: minor
Result: v1.0.0 → v1.1.0
```

#### Using Conventional Commits
```
PR Title: "feat: add dark mode support"
Result: v1.0.0 → v1.1.0

PR Title: "fix: resolve login bug"
Result: v1.0.0 → v1.0.1

PR Title: "feat!: redesign database schema"
Result: v1.0.0 → v2.0.0
```

#### Skip Release
```
PR Title: "docs: update README"
Label: skip-release
Result: No release created
```

## Release Artifacts

Each release includes:
- **Source code** (automatically included by GitHub)
- **DokAssist.dmg** - macOS disk image for easy installation
- **DokAssist.app.tar.gz** - macOS application bundle

## First Release

If no previous tags exist, the workflow starts at `v0.1.0` for the first release.

## Workflow Files

- `.github/workflows/release.yml` - Main release workflow

## Manual Release (if needed)

While the automated pipeline is recommended, you can manually create a release:

```bash
# Create and push a tag
git tag v1.0.0
git push origin v1.0.0

# The build workflow will not run automatically for manual tags
# You would need to manually trigger the build or create the release through GitHub UI
```

## Troubleshooting

### Release didn't trigger
- Ensure the PR was merged to the `main` branch
- Check if the PR had the `skip-release` label
- Verify the workflow permissions in repository settings

### Build failed
- Check the Actions tab for detailed logs
- Ensure all dependencies are properly configured
- Verify Tauri build requirements are met on macOS runner

# Automation Workflows

This repository includes several automation workflows to streamline development and release processes.

## Auto-labeling Workflow

### Overview
When a PR is opened or reopened, the auto-labeling workflow automatically applies the `autorelease` and `merge when ready` labels to enable automated release creation and PR merging.

### Default Labels Applied
- **`autorelease`** - Indicates that a release will be created when the PR is merged
- **`merge when ready`** - Enables automatic merging when all checks pass

### How to Customize
You can remove or modify these labels after the PR is created:
- **Remove `merge when ready`** if you want to merge manually
- **Remove `autorelease`** if you don't want the informational comment (release still happens by default)
- **Add version labels** (`major`, `minor`, `patch`) to control the version bump
- **Add `skip-release`** to prevent any release from being created

### Workflow File
- `.github/workflows/auto-label.yml`

## Auto-merge Workflow

### Overview
The auto-merge workflow automatically merges PRs that have the `merge when ready` label once all checks pass.

### How to Use
1. Add the `merge when ready` label to your PR
2. The workflow will monitor the PR's check status
3. Once all required checks pass and the PR is in a mergeable state, it will be automatically merged using squash merge

### Requirements
- PR must have the `merge when ready` label
- All required status checks must pass
- PR must be in a mergeable state (no conflicts, approved if required)

### Workflow File
- `.github/workflows/auto-merge.yml`

### Events That Trigger Auto-merge
- When the label is added to a PR
- When PR code is updated (synchronize)
- When check suites complete
- When status checks update

## Auto-release Workflow

### Overview
The `autorelease` label provides an explicit marker that a PR is intended to trigger a release. When a PR with this label is merged, a helpful comment is added with information about the upcoming release.

### How to Use
1. Add the `autorelease` label to your PR along with a version bump label (`major`, `minor`, or `patch`)
2. When the PR is merged, a comment will be posted with release information
3. The main release workflow (which runs on every push to `main`) will create the release automatically

### Note
The `autorelease` label is informational - releases happen automatically for all merged PRs unless the `skip-release` label is present. Use `autorelease` to clearly communicate intent and get a helpful comment about the release process.

### Workflow File
- `.github/workflows/auto-release.yml`

## Main Release Workflow

The main release workflow runs automatically on every push to `main` and creates semantic versioned releases. See [RELEASE.md](RELEASE.md) for complete documentation.

### Version Bump Priority
1. PR labels: `major`, `minor`, `patch`, `skip-release` (highest priority)
2. Conventional commit format in PR title
3. Default: patch bump

## Repository Settings

To enable these workflows, ensure the following repository settings are configured:

### Required Settings for Auto-merge
1. Go to **Settings** → **Actions** → **General**
2. Under "Workflow permissions", ensure "Read and write permissions" is selected
3. Check "Allow GitHub Actions to create and approve pull requests"

### Optional: Personal Access Token (GH_PAT)
For repositories with branch protection rules or rulesets that prevent the default GITHUB_TOKEN from pushing to main:
1. Create a Personal Access Token (classic) with `repo` scope
2. Go to **Settings** → **Secrets and variables** → **Actions**
3. Add a new repository secret named `GH_PAT` with your token
4. The release and auto-merge workflows will automatically use this token when available

**Note:** The workflows use `secrets.GH_PAT || secrets.GITHUB_TOKEN` as a fallback pattern, so GH_PAT is optional but recommended for repositories with strict branch protection.

### Optional: Branch Protection
For additional safety, consider enabling branch protection on `main`:
1. Go to **Settings** → **Branches**
2. Add a branch protection rule for `main`
3. Enable "Require status checks to pass before merging"
4. Select which checks must pass (e.g., tests, linting)
5. Enable "Require branches to be up to date before merging" (optional)

### Optional: Auto-merge Settings
1. Go to **Settings** → **General**
2. Scroll to "Pull Requests"
3. Ensure "Allow auto-merge" is enabled

## Summary of Labels

All PRs are automatically labeled with `autorelease` and `merge when ready` when opened. You can customize or remove these labels as needed.

| Label | Purpose | Effect | Applied By |
|-------|---------|--------|------------|
| `merge when ready` | Auto-merge | PR will be automatically merged when checks pass | Auto-label workflow (default) |
| `autorelease` | Release intent | Adds informational comment; release happens automatically anyway | Auto-label workflow (default) |
| `major` | Version bump | Creates a major version bump (e.g., v1.0.0 → v2.0.0) | Manual |
| `minor` | Version bump | Creates a minor version bump (e.g., v1.0.0 → v1.1.0) | Manual |
| `patch` | Version bump | Creates a patch version bump (e.g., v1.0.0 → v1.0.1) | Manual |
| `skip-release` | Skip release | Prevents automatic release creation | Manual |

## Troubleshooting

### Auto-merge not working
- Verify the `merge when ready` label is applied
- Check that all required status checks have passed
- Ensure the PR has no merge conflicts
- Verify repository settings allow GitHub Actions to merge PRs
- Check the Actions tab for any workflow errors

### Release not created
- Ensure the `skip-release` label is not present
- Check the [release workflow](https://github.com/vGsteiger/IbexDoc/actions/workflows/release.yml) in the Actions tab
- Verify the PR was successfully merged to `main`
- Check that the workflow has necessary permissions

### Permission errors when pushing version commits or tags
- This typically occurs when branch protection or rulesets prevent the default GITHUB_TOKEN from pushing
- Add a `GH_PAT` secret (Personal Access Token with `repo` scope) to your repository
- Ensure the PAT has an exception in your branch protection rules or rulesets
- The release workflow will automatically use GH_PAT when available

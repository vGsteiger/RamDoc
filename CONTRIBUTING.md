# Contributing to DokAssist

Thank you for your interest in contributing! This guide covers everything you need to get started.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Branch Naming](#branch-naming)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [First Contribution](#first-contribution)

## Getting Started

1. Fork the repository and clone your fork:
   ```bash
   git clone https://github.com/<your-username>/IbexDoc.git
   cd IbexDoc
   ```

2. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/vGsteiger/IbexDoc.git
   ```

## Development Setup

### Prerequisites

| Tool | Minimum Version |
|------|----------------|
| Rust | 1.82 (MSRV) |
| Node.js | 20 |
| pnpm | 8 |
| macOS | 13 (Ventura) |

macOS 13+ is required for full Keychain integration and Metal GPU acceleration for the local LLM.

### Install and run

```bash
cd dokassist

# Install Node dependencies
pnpm install

# Start the app in development mode (hot-reload for frontend + Rust rebuild on changes)
pnpm tauri dev
```

## Code Style

### Rust

- Formatting: `rustfmt` — run `cargo fmt` before committing
- Linting: `cargo clippy -- -D warnings` must pass with zero warnings
- Both are enforced by `lint.yml` on every push/PR

### Frontend (Svelte / TypeScript)

- Formatting: Prettier (config in `dokassist/.prettierrc` if present, else defaults)
- Linting: ESLint — run `pnpm lint` before committing
- Also enforced by `lint.yml`

## Branch Naming

```
<type>/<short-description>
```

Examples:

| Branch | When to use |
|--------|-------------|
| `feat/add-export` | New feature |
| `fix/crash-on-lock` | Bug fix |
| `docs/update-readme` | Documentation only |
| `chore/bump-deps` | Dependency updates, tooling |
| `refactor/split-commands` | Internal refactor, no behaviour change |

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <short description>

<optional body>
```

Recognised types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`

Breaking changes: append `!` after the type (e.g., `feat!: change API shape`) or add a
`BREAKING CHANGE:` footer. This triggers a major version bump.

These prefixes drive automated semantic versioning in `release.yml` when no PR label is set.

## Pull Request Process

1. **Create a PR** against `main` from your feature branch.
2. **Apply a version label** to control the release bump:

   | Label | Effect |
   |-------|--------|
   | `major` | Breaking change — bumps `X.0.0` |
   | `minor` | New feature — bumps `0.X.0` |
   | `patch` | Bug fix / chore — bumps `0.0.X` |
   | `skip-release` | Merges without cutting a release |

3. **Fill in the PR template** checklist completely.
4. **All CI checks must pass**: Rust tests, frontend tests, Clippy, security audit, build.
5. **Add the `merge when ready` label** to enable auto-merge once approved (requires all checks green and no `CHANGES_REQUESTED` reviews).
6. A maintainer will review and merge. Single-reviewer approval is sufficient for non-breaking changes.

## Testing Requirements

All tests must pass before a PR can merge.

### Rust (86 unit tests)

```bash
cd dokassist/src-tauri
cargo test --lib
```

### Frontend (Vitest)

```bash
cd dokassist
pnpm test

# With coverage report
pnpm test:coverage
```

New code should include tests for non-trivial logic. Security-sensitive code (crypto, auth, input sanitisation) must have tests.

## First Contribution

Look for issues labelled [`good first issue`](https://github.com/vGsteiger/IbexDoc/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) — these are scoped tasks suitable for new contributors.

If you want to work on something not yet filed as an issue, open one first to discuss the approach before investing time in implementation.

## Code of Conduct

All contributors are expected to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

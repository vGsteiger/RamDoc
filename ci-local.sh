#!/usr/bin/env bash
# Local CI — mirrors the GitHub Actions workflows:
#   rust-ci.yml, frontend-ci.yml, lint.yml, security.yml
#
# Usage:
#   ./ci-local.sh            # run all checks
#   ./ci-local.sh --rust     # Rust checks only
#   ./ci-local.sh --frontend # Frontend checks only
#   ./ci-local.sh --lint     # Lint/format checks only
#   ./ci-local.sh --security # Security audit (requires cargo-audit / cargo-deny)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="$REPO_ROOT/dokassist/src-tauri"
FRONTEND_DIR="$REPO_ROOT/dokassist"

# ── Colours ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BOLD='\033[1m'; RESET='\033[0m'

pass() { echo -e "  ${GREEN}✓${RESET} $1"; }
fail() { echo -e "  ${RED}✗${RESET} $1"; }
warn() { echo -e "  ${YELLOW}~${RESET} $1 (skipped)"; }
section() { echo -e "\n${BOLD}━━ $1 ━━${RESET}"; }

# ── Result tracking ──────────────────────────────────────────────────────────
RESULTS=()   # "label:status"  status = pass | fail | skip

record() {   # record <label> <exit-code>
  local label="$1" code="$2"
  if   [[ $code -eq 0 ]];   then RESULTS+=("$label:pass")
  elif [[ $code -eq 99 ]];  then RESULTS+=("$label:skip")
  else                            RESULTS+=("$label:fail")
  fi
}

run_step() {  # run_step <label> <cmd…>
  local label="$1"; shift
  echo -ne "  ${BOLD}→${RESET} $label … "
  if "$@" >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "$label" 0
  else
    echo -e "${RED}FAILED${RESET}"
    echo "---- output ----"
    cat "$TMPLOG"
    echo "----------------"
    record "$label" 1
    OVERALL=1
  fi
}

run_optional() {  # run_optional <tool> <label> <cmd…>
  local tool="$1" label="$2"; shift 2
  if ! command -v "$tool" &>/dev/null; then
    echo -e "  ${YELLOW}~${RESET} $label (${tool} not installed — skipping)"
    record "$label" 99
    return
  fi
  run_step "$label" "$@"
}

TMPLOG="$(mktemp)"
trap 'rm -f "$TMPLOG"' EXIT
OVERALL=0

# ── Filter flags ─────────────────────────────────────────────────────────────
RUN_RUST=1; RUN_FRONTEND=1; RUN_LINT=1; RUN_SECURITY=1
if [[ $# -gt 0 ]]; then
  RUN_RUST=0; RUN_FRONTEND=0; RUN_LINT=0; RUN_SECURITY=0
  for arg in "$@"; do
    case "$arg" in
      --rust)     RUN_RUST=1 ;;
      --frontend) RUN_FRONTEND=1 ;;
      --lint)     RUN_LINT=1 ;;
      --security) RUN_SECURITY=1 ;;
      *) echo "Unknown flag: $arg"; exit 1 ;;
    esac
  done
fi

# ════════════════════════════════════════════════════════════════════════════
# RUST CI  (rust-ci.yml)
# ════════════════════════════════════════════════════════════════════════════
if [[ $RUN_RUST -eq 1 ]]; then
  section "Rust CI"
  cd "$TAURI_DIR"

  run_step  "cargo check"   cargo check --all-targets
  run_step  "cargo test"    cargo test --lib

  # Example — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} cargo run --example test_audit … "
  if cargo run --example test_audit >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "cargo example" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    record "cargo example" 0   # mirrors continue-on-error: true
  fi

  run_step  "cargo clippy"  cargo clippy --all-targets --all-features -- \
      -D warnings \
      -A clippy::too_many_arguments \
      -A clippy::type_complexity

  run_step  "cargo fmt"     cargo fmt -- --check

  cd "$REPO_ROOT"
fi

# ════════════════════════════════════════════════════════════════════════════
# FRONTEND CI  (frontend-ci.yml)
# ════════════════════════════════════════════════════════════════════════════
if [[ $RUN_FRONTEND -eq 1 ]]; then
  section "Frontend CI"
  cd "$FRONTEND_DIR"

  run_step "pnpm install"  pnpm install --frozen-lockfile
  run_step "pnpm build"    pnpm build
  run_step "pnpm test"     pnpm test

  # svelte-check — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} svelte-check … "
  if pnpm list svelte-check &>/dev/null && \
     pnpm exec svelte-check --tsconfig ./tsconfig.json >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "svelte-check" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    cat "$TMPLOG"
    record "svelte-check" 0   # mirrors continue-on-error: true
  fi

  cd "$REPO_ROOT"
fi

# ════════════════════════════════════════════════════════════════════════════
# LINT  (lint.yml)
# ════════════════════════════════════════════════════════════════════════════
if [[ $RUN_LINT -eq 1 ]]; then
  section "Lint & Format"
  cd "$FRONTEND_DIR"

  # Prettier — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} prettier … "
  if pnpm list prettier &>/dev/null && \
     pnpm exec prettier --check "src/**/*.{js,ts,svelte,json,css}" >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "prettier" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    cat "$TMPLOG"
    record "prettier" 0   # mirrors continue-on-error: true
  fi

  # ESLint — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} eslint … "
  if pnpm list eslint &>/dev/null && \
     pnpm exec eslint "src/**/*.{js,ts,svelte}" >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "eslint" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    cat "$TMPLOG"
    record "eslint" 0     # mirrors continue-on-error: true
  fi

  cd "$REPO_ROOT"

  # typos — continue-on-error in CI, skipped if not installed
  echo -ne "  ${BOLD}→${RESET} typos … "
  if ! command -v typos &>/dev/null; then
    echo -e "${YELLOW}skipped${RESET} (typos not installed — run: cargo install typos-cli)"
    record "typos" 99
  elif typos ./dokassist/src-tauri/src ./dokassist/src README.md >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "typos" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    cat "$TMPLOG"
    record "typos" 0      # mirrors continue-on-error: true
  fi
fi

# ════════════════════════════════════════════════════════════════════════════
# SECURITY  (security.yml)
# ════════════════════════════════════════════════════════════════════════════
if [[ $RUN_SECURITY -eq 1 ]]; then
  section "Security Audit"
  cd "$TAURI_DIR"

  # cargo audit — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} cargo audit … "
  if ! command -v cargo-audit &>/dev/null; then
    echo -e "${YELLOW}skipped${RESET} (not installed — run: cargo install cargo-audit)"
    record "cargo audit" 99
  elif cargo audit --deny warnings >"$TMPLOG" 2>&1; then
    echo -e "${GREEN}passed${RESET}"
    record "cargo audit" 0
  else
    echo -e "${YELLOW}warned${RESET} (non-blocking)"
    cat "$TMPLOG"
    record "cargo audit" 0    # mirrors continue-on-error: true
  fi

  # cargo deny — continue-on-error in CI
  echo -ne "  ${BOLD}→${RESET} cargo deny … "
  if ! command -v cargo-deny &>/dev/null; then
    echo -e "${YELLOW}skipped${RESET} (not installed — run: cargo install cargo-deny)"
    record "cargo deny" 99
  else
    # Create deny.toml if absent (mirrors the CI step)
    if [[ ! -f deny.toml ]]; then
      cat > deny.toml << 'TOML'
[advisories]
version = 2
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
ignore = []

[licenses]
version = 2
allow = [
  "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause", "BSD-3-Clause", "ISC",
  "Unicode-DFS-2016", "Unlicense", "Zlib",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
TOML
    fi
    if cargo deny check >"$TMPLOG" 2>&1; then
      echo -e "${GREEN}passed${RESET}"
      record "cargo deny" 0
    else
      echo -e "${YELLOW}warned${RESET} (non-blocking)"
      cat "$TMPLOG"
      record "cargo deny" 0   # mirrors continue-on-error: true
    fi
  fi

  cd "$REPO_ROOT"
fi

# ════════════════════════════════════════════════════════════════════════════
# SUMMARY
# ════════════════════════════════════════════════════════════════════════════
section "Summary"
PASSED=0; FAILED=0; SKIPPED=0
for entry in "${RESULTS[@]}"; do
  label="${entry%:*}"; status="${entry##*:}"
  case "$status" in
    pass) pass "$label"; ((PASSED++)) ;;
    fail) fail "$label"; ((FAILED++)) ;;
    skip) warn "$label"; ((SKIPPED++)) ;;
  esac
done

echo ""
echo -e "  ${BOLD}${PASSED} passed · ${FAILED} failed · ${SKIPPED} skipped${RESET}"

if [[ $OVERALL -ne 0 ]]; then
  echo -e "\n  ${RED}${BOLD}CI FAILED${RESET}"
  exit 1
else
  echo -e "\n  ${GREEN}${BOLD}CI PASSED${RESET}"
fi

#!/usr/bin/env node
/**
 * Builds the out-of-process speech-to-text worker (`asr-worker/`, binary
 * `dokassist-asr`) used by dictation.
 *
 *   node scripts/build-asr-worker.mjs           # release build + stage as a Tauri sidecar
 *   node scripts/build-asr-worker.mjs --dev     # build only; the debug app finds it in asr-worker/target
 *
 * Staging copies the binary to src-tauri/binaries/dokassist-asr-<target-triple>,
 * the name Tauri's `bundle.externalBin` expects. That externalBin entry lives in
 * src-tauri/tauri.bundle.conf.json (passed with `tauri build --config`), so plain
 * `cargo build` / `cargo test` / clippy never require the sidecar to exist.
 *
 * Why a separate process: see asr-worker/Cargo.toml.
 */
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, chmodSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;
const WORKER_DIR = join(ROOT, 'asr-worker');
const NAME = 'dokassist-asr';
const dev = process.argv.includes('--dev');

function hostTriple() {
  const out = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
  const line = out.split('\n').find((l) => l.startsWith('host:'));
  if (!line) throw new Error('could not determine host target triple from `rustc -vV`');
  return line.slice('host:'.length).trim();
}

// Tauri exports the target of the app build to before*Command hooks.
const triple = process.env.TAURI_ENV_TARGET_TRIPLE || hostTriple();
const args = ['build', '--release', '--manifest-path', join(WORKER_DIR, 'Cargo.toml')];
if (!dev) args.push('--target', triple);

console.log(`[asr-worker] cargo ${args.join(' ')}`);
execFileSync('cargo', args, { stdio: 'inherit', cwd: WORKER_DIR });

if (!dev) {
  const built = join(WORKER_DIR, 'target', triple, 'release', NAME);
  const stagedDir = join(ROOT, 'src-tauri', 'binaries');
  const staged = join(stagedDir, `${NAME}-${triple}`);
  mkdirSync(stagedDir, { recursive: true });
  copyFileSync(built, staged);
  chmodSync(staged, 0o755);
  console.log(`[asr-worker] staged sidecar ${staged}`);
}

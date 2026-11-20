#!/usr/bin/env node
/**
 * Guards the translation layer:
 *
 *   1. de.json and en.json hold exactly the same keys, and every leaf is a
 *      string. `getNestedValue` in $lib/translations only returns strings, so
 *      an array or number leaf is silently unreachable.
 *   2. No new hardcoded user-visible string in .svelte markup. Text nodes and
 *      the user-visible attributes have to come from `$t(...)`.
 *   3. No string literal that can be *output* by a `{…}` text expression,
 *      e.g. `{count === 1 ? 'measurement' : 'measurements'}` or
 *      `{name || 'Unknown'}`. See `textExpressionLiterals` for what counts.
 *
 * The second check is a heuristic, so genuinely untranslatable literals are
 * listed in ALLOW below with a stated reason. Anything not listed fails.
 *
 * Run: node scripts/check-i18n.mjs
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = new URL('..', import.meta.url).pathname;
const SRC = join(ROOT, 'src');

/* ------------------------------------------------------------------ *
 * Untranslatable by nature. Each entry needs a reason.
 * ------------------------------------------------------------------ */
const ALLOW = [
  // Language names are always written in their own language.
  'English',
  'Deutsch',
  'Français',
  // Product name.
  'RamDoc',
  // Keyboard shortcuts are not localised.
  'Cmd+K',
  'Cmd+N',
  'Cmd+Shift+S',
  'Esc',
  // The literal word the user must type; the handler string-compares it.
  'RESTORE',
  // Model identifier.
  'nomic-embed-text-v1.5',
  // Input format hints, not prose.
  '+41 XX XXX XX XX',
  '756.____.____.__',
  'practice@example.com',
  'recipient@example.com',
  // The required-field marker rendered by ui/Field.
  '&nbsp;*',
  // Identical in both languages, and shown as a prefix to a value.
  'ATC:',
  'AHV:',
  // The authority and database behind the medication reference, a proper noun.
  // The "Quelle:" / "Source:" prefix around it is translated.
  ': Swissmedic AIPS',
];

// Text that carries no words to translate: punctuation, symbols, numbers,
// short acronyms and units, and anything that looks like a CSS class. Units
// matter now that interpolations are stripped — `{n} min` reduces to `min`.
const NOISE =
  /^(?:[\s—•·▸▾▶▼●✓→←↑↓↵|/\\:;,.\-–+*#%()[\]{}0-9]+|[A-Z]{2,5}|[A-Za-zÄÖÜäöü]{1,3}\.?|https?:.*|&nbsp;|tok\/s|[a-z-]+(?:\s[a-z-]+)*=.*)$/;

// Quoted attribute values, including ones that interpolate. A template such as
// aria-label="Session with {name}, status: {status}" is still hardcoded English
// prose — skipping any value containing braces is how four of them survived the
// first sweep. Interpolations are blanked before the value is judged, so a
// fully-dynamic aria-label={...} is unaffected (it has no quoted value at all).
const ATTRS = /\b(placeholder|aria-label|title|alt|label|description|hint)="([^"]+)"/g;
const INTERPOLATION = /\{[^{}]*\}/g;

function walk(dir) {
  return readdirSync(dir).flatMap((entry) => {
    const full = join(dir, entry);
    return statSync(full).isDirectory() ? walk(full) : [full];
  });
}

function flatten(obj, prefix = '') {
  return Object.entries(obj).flatMap(([key, value]) =>
    // Plain objects are namespaces and get walked. Arrays must NOT be — an
    // array leaf would otherwise flatten into `key.0`, `key.1`, … and look
    // valid, when getNestedValue cannot reach it at all.
    value && typeof value === 'object' && !Array.isArray(value)
      ? flatten(value, `${prefix}${key}.`)
      : [[`${prefix}${key}`, value]]
  );
}

/**
 * Blanks out everything that isn't markup — script blocks, style blocks and
 * comments — replacing each with the same number of newlines so reported line
 * numbers still line up with the file.
 *
 * Tag matching is case-insensitive and comment stripping repeats until the
 * source stops changing. This scans our own tree rather than untrusted input,
 * but a single pass over a case-sensitive pattern is the classic sanitiser
 * mistake, and here it would let a hardcoded string hide from the check.
 */
function stripNonMarkup(source) {
  const blank = (m) => '\n'.repeat(m.split('\n').length - 1);
  // A closing tag may carry whitespace and junk before its `>` — a parser
  // accepts `</script\t\n foo>` as the end of the block, so the pattern has to
  // as well, or the script body would be treated as markup. The `\s` keeps
  // `</scriptfoo>`, which is not a closing tag, from matching.
  let out = source.replace(/<script\b[\s\S]*?<\/script(?:\s[^>]*)?>/gi, blank);
  out = out.replace(/<style\b[\s\S]*?<\/style(?:\s[^>]*)?>/gi, blank);
  let previous;
  do {
    previous = out;
    out = out.replace(/<!--[\s\S]*?-->/g, blank);
  } while (out !== previous);
  // An unterminated comment would otherwise leave its body in the markup.
  return out.replace(/<!--[\s\S]*$/, blank);
}

/**
 * Removes every `{…}` region by counting brace depth, keeping newlines so line
 * numbers survive. Depth counting rather than a regex because expressions nest
 * (`{`+ ${$t('…')}`}`) and span lines.
 *
 * This is what lets a *partly* interpolated text node be judged on its literal
 * words: `Von {a} bis {b}` reduces to `Von bis`, which is hardcoded German that
 * a `[^{}]`-based matcher skips entirely. It also drops `{@const …}` blocks,
 * whose `>=` would otherwise look like the end of a tag.
 */
function stripExpressions(markup) {
  let out = '';
  let depth = 0;
  for (const char of markup) {
    if (char === '{') depth++;
    else if (char === '}') depth = Math.max(0, depth - 1);
    else if (!depth || char === '\n') out += char;
  }
  return out;
}

/**
 * Finds string literals that a `{…}` *text* expression can render, returning
 * `{ line, value }` for each. Runs on markup after stripNonMarkup.
 *
 * Only expressions in text position count: attribute expressions
 * (`class={…}`) are skipped by tracking whether we are inside a tag, and
 * block/special tags (`{#if}`, `{:else}`, `{/each}`, `{@render}`, `{@const}`)
 * are skipped by their sigil.
 *
 * Within an expression a literal is treated as data, not output, when it is:
 *   - a call or array argument: preceded by `(`, `,` or `[` (`$t('a.b')`,
 *     `.join(', ')`, `fn(x, 'y')`)
 *   - one side of a comparison (`===`, `!==`, `==`, `!=`)
 *   - an index (`obj['x']`)
 * Template literals are judged on their text with `${…}` removed. Anything
 * left is judged by the same rules as a text node (NOISE, ALLOW, length).
 * Exported for the guard's own tests.
 */
export function textExpressionLiterals(markup) {
  const found = [];
  let depth = 0;
  let inTag = false;
  let start = -1;
  for (let i = 0; i < markup.length; i++) {
    const char = markup[i];
    if (depth === 0) {
      if (char === '<' && /[A-Za-z/!]/.test(markup[i + 1] ?? '')) inTag = true;
      else if (char === '>' && inTag) inTag = false;
      else if (char === '{') {
        depth = 1;
        start = i;
      }
      continue;
    }
    if (char === '{') depth++;
    else if (char === '}' && --depth === 0 && !inTag) {
      const body = markup.slice(start + 1, i);
      if (/^\s*[#:/@]/.test(body)) continue;
      const line = markup.slice(0, start).split('\n').length;
      for (const value of outputLiterals(body)) found.push({ line, value });
    }
  }
  return found;
}

const COMPARISON_BEFORE = /(?:===|!==|==|!=)\s*$/;
const COMPARISON_AFTER = /^\s*(?:===|!==|==|!=)/;
const TRANSLATION_KEY = /^[a-z][A-Za-z0-9]*(?:\.[A-Za-z0-9_]+)+$/;
const STRING_LITERAL = /(['"`])((?:\\.|(?!\1)[^\\])*)\1/g;

function outputLiterals(expression) {
  const values = [];
  for (const m of expression.matchAll(STRING_LITERAL)) {
    const before = expression.slice(0, m.index).trimEnd();
    const after = expression.slice(m.index + m[0].length);
    const prev = before.slice(-1);
    if (prev === '(' || prev === ',' || prev === '[') continue;
    if (COMPARISON_BEFORE.test(before) || COMPARISON_AFTER.test(after)) continue;
    if (/^\s*\]/.test(after)) continue;
    // A translation key chosen by a ternary, `$t(ok ? 'a.yes' : 'a.no')`.
    if (TRANSLATION_KEY.test(m[2])) continue;
    values.push(m[1] === '`' ? m[2].replace(/\$\{[^}]*\}/g, ' ') : m[2]);
  }
  return values;
}

function main() {
  const problems = [];

  /* --------------------------- 1. bundle parity --------------------------- */
  const de = JSON.parse(readFileSync(join(SRC, 'lib/translations/de.json'), 'utf8'));
  const en = JSON.parse(readFileSync(join(SRC, 'lib/translations/en.json'), 'utf8'));
  const deEntries = flatten(de);
  const enEntries = flatten(en);
  const deKeys = new Set(deEntries.map(([k]) => k));
  const enKeys = new Set(enEntries.map(([k]) => k));

  for (const key of deKeys) {
    if (!enKeys.has(key)) problems.push(`en.json is missing key: ${key}`);
  }
  for (const key of enKeys) {
    if (!deKeys.has(key)) problems.push(`de.json is missing key: ${key}`);
  }
  for (const [lang, entries] of [
    ['de', deEntries],
    ['en', enEntries],
  ]) {
    for (const [key, value] of entries) {
      if (typeof value !== 'string') {
        const kind = Array.isArray(value) ? 'an array' : `a ${typeof value}`;
        problems.push(`${lang}.json: ${key} is ${kind}, but only strings are reachable`);
      }
    }
  }

  /* ----------------------- 2. hardcoded UI strings ----------------------- */
  for (const file of walk(SRC)) {
    if (!file.endsWith('.svelte')) continue;
    const rel = relative(ROOT, file);
    if (rel.startsWith('src/tests/')) continue;

    // Expressions go first, so a partly interpolated text node is still judged
    // on the literal words around the interpolation.
    const stripped = stripNonMarkup(readFileSync(file, 'utf8'));
    const markup = stripExpressions(stripped);

    const report = (line, kind, text) => {
      const value = text.replace(/\s+/g, ' ').trim();
      if (value.length < 3) return;
      if (!/[A-Za-zÄÖÜäöü]{3}/.test(value)) return;
      if (NOISE.test(value)) return;
      if (ALLOW.includes(value)) return;
      problems.push(`${rel}:${line}  hardcoded ${kind}: ${value}`);
    };

    for (const m of markup.matchAll(/>([^<>]+)</g)) {
      report(markup.slice(0, m.index).split('\n').length, 'text', m[1]);
    }
    markup.split('\n').forEach((line, i) => {
      for (const m of line.matchAll(ATTRS)) {
        report(i + 1, m[1], m[2].replace(INTERPOLATION, ' '));
      }
    });

    /* --------------------- 3. literals in {…} text --------------------- */
    for (const { line, value } of textExpressionLiterals(stripped)) {
      report(line, 'string in {…} expression', value);
    }
  }

  if (problems.length) {
    console.error(`\n✖ ${problems.length} i18n problem(s):\n`);
    for (const p of problems) console.error(`  ${p}`);
    console.error(`
User-visible text belongs in src/lib/translations/{de,en}.json and is read with
$t('namespace.key') — see the existing namespaces for where a new key fits.
Values that are persisted (session types, report types, status slugs) keep their
stored form and are translated for display only; see src/lib/translations/labels.ts.
If a literal genuinely cannot be translated, add it to ALLOW in
scripts/check-i18n.mjs with a reason.
`);
    process.exit(1);
  }

  console.log(`✔ de/en at ${deKeys.size} keys with no drift, and no hardcoded UI strings in src/`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();

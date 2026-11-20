#!/usr/bin/env node
/**
 * Fails if a view reintroduces a raw Tailwind palette utility, a dark: colour
 * variant, a radius/shadow from Tailwind's default scale, a type size outside
 * the six-step scale, font-bold, or a raw black/white scrim. All of them
 * hardcode a decision at the call site, which is exactly the duplication the
 * token layer in src/app.css exists to remove.
 *
 * Run: node scripts/check-design-tokens.mjs
 * See: docs/design-language.md
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = new URL('..', import.meta.url).pathname;
const SRC = join(ROOT, 'src');

const HUES =
  'gray|slate|zinc|neutral|stone|blue|indigo|sky|cyan|purple|violet|fuchsia|pink|red|rose|green|emerald|teal|lime|amber|yellow|orange';
const PROPS =
  'bg|text|border|ring|divide|placeholder|from|via|to|outline|decoration|caret|fill|stroke';

// Deliberately property-agnostic: match ANY utility ending in -<hue>-<shade>,
// whatever prefixes it. Enumerating properties previously let compound ones
// through — `ring-offset-gray-900` is not `ring-<hue>`, so two real violations
// survived the first sweep.
const RAW_PALETTE = new RegExp(
  `(?:^|[\\s"'\`{}])[\\w:./-]*?-(?:${HUES})-(?:50|[1-9]00|950)(?:/\\d{1,3})?(?![\\w-])`,
  'g'
);
// Raw white/black are equally theme-hardcoded — bare or with an opacity
// modifier. Scrims used to be the one exception (`bg-black/50`); they are now
// the `bg-scrim` / `bg-scrim-strong` tokens, which differ per theme.
const RAW_NEUTRAL = new RegExp(
  `(?:^|[\\s"'\`{}])[\\w:./-]*?-(?:white|black)(?:/\\d{1,3})?(?![\\w/-])`,
  'g'
);
// The type scale is six named steps (text-display … text-caption). Tailwind's
// default sizes are a seventh scale by another name, and `text-md` does not
// exist at all — it silently falls back to the inherited size.
const RAW_TEXT_SIZE =
  /(?:^|[\s"'`{}])(?:[\w-]+:)*text-(?:xs|sm|base|md|lg|xl|[2-9]xl)(?:\/[\w.[\]-]+)?(?![\w-])/g;
// Weight stops at font-semibold. Hierarchy comes from size and colour.
const HEAVY_WEIGHT = /(?:^|[\s"'`{}])(?:[\w-]+:)*font-(?:bold|extrabold|black)(?![\w-])/g;
const DEAD_OPACITY = /(?:^|[\s"'`{}])(?:bg|text|border|ring|divide)-opacity-\d+(?![\w-])/g;
const DARK_VARIANT = new RegExp(
  `(?:^|[\\s"'\`{}])dark:(?!prose-invert)(?:[a-z-]+:)*(?:${PROPS})-`,
  'g'
);
// Geometry is two radii — rounded-control and rounded-card — plus rounded-full
// for avatars and dots. Anything on Tailwind's default scale is a third radius,
// and the corner variants (rounded-bl-sm) are the ones that slip through review.
// Bare `rounded` counts: it silently means 4px.
const RAW_RADIUS =
  /(?:^|[\s"'`{}])rounded(?:-(?:[tblrse]|tl|tr|bl|br|ss|se|es|ee))?(?:-(?:none|sm|md|lg|xl|2xl|3xl|4xl))?(?![\w-])/g;
// Elevation is shadow-popover and shadow-modal. Nothing else, and never on
// inline content — cards and rows separate with a hairline border.
const RAW_SHADOW = /(?:^|[\s"'`{}])shadow(?:-(?:2xs|xs|sm|md|lg|xl|2xl|inner|none))?(?![\w-])/g;

// Narrow, deliberate exceptions, matched against the whole source line.
// Anything added here needs a stated reason.
// Each entry is { file, line: RegExp }. There are currently none: the modal
// scrim, the former exception, is now a token in app.css.
const ALLOW = [];

const RULES = [
  ['raw palette utility', RAW_PALETTE],
  ['raw white/black (use bg-scrim / bg-scrim-strong for overlays)', RAW_NEUTRAL],
  ['removed in Tailwind v4', DEAD_OPACITY],
  ['dark: colour variant', DARK_VARIANT],
  ['radius outside the two-radius scale', RAW_RADIUS],
  ['shadow outside the elevation tokens', RAW_SHADOW],
  ['type size outside the scale (text-display|title|heading|body|label|caption)', RAW_TEXT_SIZE],
  ['weight above font-semibold', HEAVY_WEIGHT],
];

/**
 * Returns `[label, match]` for every violation on one source line. Exported
 * for the guard's own tests (src/tests/unit/check-design-tokens.test.ts).
 */
export function lineViolations(line) {
  // Comments necessarily name the utilities they tell you not to write. `/*`
  // matters as much as `*` and `//`: it is the line the prose starts on.
  const trimmed = line.trimStart();
  if (trimmed.startsWith('*') || trimmed.startsWith('//') || trimmed.startsWith('/*')) return [];
  const found = [];
  for (const [label, re] of RULES) {
    for (const m of line.matchAll(re)) found.push([label, m[0].trim().replace(/^["'`{}]/, '')]);
  }
  return found;
}

function walk(dir) {
  return readdirSync(dir).flatMap((entry) => {
    const full = join(dir, entry);
    return statSync(full).isDirectory() ? walk(full) : [full];
  });
}

function main() {
  const violations = [];

  for (const file of walk(SRC)) {
    if (!/\.(svelte|ts)$/.test(file)) continue;
    const rel = relative(ROOT, file);
    // The token definitions themselves, and the doc comment describing what not
    // to write, necessarily name raw utilities.
    if (rel === 'src/app.css' || rel === 'src/lib/components/ui/index.ts') continue;
    if (rel.startsWith('src/tests/')) continue;

    const lines = readFileSync(file, 'utf8').split('\n');
    lines.forEach((line, i) => {
      if (ALLOW.some((a) => a.file === rel && a.line.test(line))) return;
      for (const [label, match] of lineViolations(line)) {
        violations.push(`${rel}:${i + 1}  ${label}: ${match}`);
      }
    });
  }

  if (violations.length) {
    console.error(`\n✖ ${violations.length} design-token violation(s):\n`);
    for (const v of violations) console.error(`  ${v}`);
    console.error(`
Views must use the semantic tokens from src/app.css — bg-surface, bg-surface-raised,
text-fg, text-fg-muted, border-line, bg-accent, text-danger-fg, … — and compose the
primitives in src/lib/components/ui/. Tokens theme themselves, so a dark: colour
variant is never needed. Geometry is rounded-control / rounded-card (plus
rounded-full for avatars and dots), elevation is shadow-popover / shadow-modal,
type is text-display|title|heading|body|label|caption with at most font-semibold,
and overlays use bg-scrim. See docs/design-language.md.
`);
    process.exit(1);
  }

  console.log('✔ src/ uses only the colour, type, radius and elevation tokens from app.css');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();

# RamDoc design language

RamDoc is a clinical record system. It should read like one: quiet, dense,
legible, and boring in the way good tools are boring. The reference points are
the Claude, Codex, Notion and Linear desktop apps — cool near-neutral surfaces,
hairline separation, ink for action, and a deliberate type scale.

The visual idea comes from the logo: **one thin, uniform red stroke on a cool
near-white tile.** The UI follows it. Everything is ink on paper; the single
coloured stroke is the brand mark and the active-navigation bar.

The rule that makes this hold: **views never name a colour.** They compose the
primitives in `src/lib/components/ui/` and the semantic token utilities defined
in `src/app.css`. A raw palette class such as `bg-gray-100` or `text-blue-600`
hardcodes a theme at the call site, which is what previously required 3420
colour decisions and 1330 `dark:` variants to stay in sync by hand.

`pnpm check:design` enforces this and runs in CI. It rejects raw palette
utilities, raw `black`/`white` (including `bg-black/50` scrims), `dark:` colour
variants, any radius or shadow from Tailwind's default scale (including corner
variants like `rounded-bl-sm` and bare `rounded`), Tailwind's default type sizes
(`text-sm`, `text-lg`, and the non-existent `text-md`), and `font-bold` and
heavier.

---

## 1. Tokens

All tokens are CSS custom properties on `:root`, re-declared on `.dark`, and
exposed to Tailwind through `@theme inline`. Because the block is `inline`,
every generated utility resolves `var()` at use time — so toggling `dark` on
`<html>` reheats the entire UI and **no `dark:` colour variant is ever needed**.

Theme switching is therefore one line, in `src/routes/+layout.svelte`:

```js
document.documentElement.classList.toggle("dark", $resolvedTheme === "dark");
```

### Surfaces (cool neutrals)

| Utility               | Light     | Dark      | Role                                         |
| --------------------- | --------- | --------- | -------------------------------------------- |
| `bg-surface`          | `#F7F7FA` | `#151518` | The page.                                    |
| `bg-surface-raised`   | `#FFFFFF` | `#1D1D21` | Cards, inputs, anything sitting on the page. |
| `bg-surface-overlay`  | `#FFFFFF` | `#232328` | Dialogs, popovers, menus, toasts.            |
| `bg-surface-sunken`   | `#F0F0F5` | `#111114` | Sidebar, wells, segmented-control tracks.    |
| `bg-surface-hover`    | `#EBEBF0` | `#26262B` | Hover state, quiet chips.                    |
| `bg-surface-selected` | `#E3E3EA` | `#2F2F35` | Selected row, active nav item, pressed.      |

`#F0F0F5` is the logo tile. The ramp is cool so the brand red reads crisply.

### Lines

`border-line-subtle` · `border-line` · `border-line-strong`

Hairlines are the primary separation mechanism. Reach for a border before a
shadow, and before a filled surface.

### Foreground

`text-fg` (primary) · `text-fg-muted` (secondary) · `text-fg-subtle` (metadata,
icons) · `text-fg-disabled`

### Accent = ink (D1)

`bg-accent` · `bg-accent-hover` · `bg-accent-subtle` · `border-accent` ·
`border-accent-line` · `text-accent-fg` · `text-on-accent`

The accent is **ink**, not a hue: `#1B1B1F` in light, inverted to `#ECECF0` (with
near-black `on-accent` text) in dark. It fills:

- the single primary action in a view (`Button variant="primary"`)
- selection indicators: the selected tab's 2px underline, the selected segment
- tooltips (inverse chip)

`accent-subtle` and `accent-line` are neutral, for selected-row tints. If two
buttons on screen are ink-filled, one of them is wrong.

### Brand

`text-brand` · `bg-brand` · `bg-brand-subtle` — the logo red `#DC0000`
(`#FF4D4D` in dark, ≥4.5:1 on the dark page).

Used **only** by `BrandMark` and the 2px active-navigation bar. Never on a
button, never as body or link text, never next to a danger element. One stroke,
one signal.

### Links and focus

- `text-link` / `hover:text-link-hover` — a muted blue (`#2C5BB8` / `#8FB0F5`).
  Links are underlined.
- `--focus` / `outline-focus` / `ring-focus` — a separate blue ring, so a
  focused field never looks like an error (red) or a selection (ink).

### Status

`danger` · `success` · `warning` · `info`, each with `bg-<tone>`,
`bg-<tone>-subtle`, `border-<tone>-line`, `text-<tone>-fg`, `text-on-<tone>`.

- **Danger** (`#C42B1C`) is a darker red, deliberately distinct from the brand
  red, reserved for destructive actions and errors. It always comes with an icon
  and a verb.
- **Info** is its own blue (it used to alias the accent), so info alerts do not
  turn ink.
- Success is a state, not an action colour: there are no green buttons.

Colour carries meaning here and nowhere else, and **never alone**: a status is
an icon plus a text label (`StatusIndicator`, `Badge`, `Alert`).

### Overlays and highlight

- `bg-scrim` — the modal/drawer wash (`rgb(20 20 28 / .28)` light,
  `rgb(0 0 0 / .55)` dark). `bg-scrim-strong` for media viewers.
  `bg-black/NN` is rejected by the guard.
- `bg-highlight` — search matches inside `<mark>` and text selection.

---

## 2. Type

System font (SF Pro on macOS). Six steps. Hierarchy comes from size and
colour, not weight.

| Utility        | Size | Use                                                  |
| -------------- | ---- | ---------------------------------------------------- |
| `text-display` | 24px | One per page: the page title (`PageShell`).          |
| `text-title`   | 18px | Card and dialog titles, large section titles.        |
| `text-heading` | 15px | Section headings (`SectionHeader`), emphasised rows. |
| `text-body`    | 14px | The default.                                         |
| `text-label`   | 13px | Form labels, dense table cells, descriptions.        |
| `text-caption` | 12px | Metadata, hints, timestamps.                         |

Weight stops at `font-semibold` (headings) and `font-medium` (emphasis);
`font-bold` is rejected by the guard. Tabular figures are on by default for
`table`, `time`, and `[data-numeric]`, so dosages and scores align in columns.

### Measure

| Utility         | Value | Use                                            |
| --------------- | ----- | ---------------------------------------------- |
| `max-w-measure` | 72ch  | Prose: notes, reports, letters, editors.       |
| `max-w-chat`    | 760px | The conversation column (messages + composer). |
| `max-w-form`    | 48rem | Settings panes and forms.                      |
| `max-w-page`    | 76rem | The wide page container (lists, records).      |

---

## 3. Geometry and size

**Two radii.** `rounded-control` (6px) for buttons, inputs, selects, chips.
`rounded-card` (10px) for cards, dialogs, popovers. `rounded-full` is reserved
for avatars, dot indicators and progress bars. Corner variants take the same
tokens — `rounded-t-card`, not `rounded-t-lg`.

**Three control heights (D5).**

| Utility        | Height | Use                                                      |
| -------------- | ------ | -------------------------------------------------------- |
| `h-control`    | 32px   | Default for every button, input, select, tab, menu item. |
| `h-control-sm` | 28px   | Dense: toolbars, list-row actions, table rows.           |
| `h-control-lg` | 36px   | **Only** the chat composer send button and auth screens. |
| `min-h-row`    | 36px   | List rows (`ListRow`).                                   |

The same names exist as `size-control*` for square icon buttons. The
primitives encode these — `Button size="sm|md|lg"`, `IconButton size`,
`Input/Select controlSize` — so don't hand-roll `h-10` or `px-6 py-3`. WCAG
2.5.8 asks for 24px targets; 28px dense controls clear it.

**Spacing** follows the Tailwind 4px scale, kept compact. Page rhythm has its
own tokens, used only by `PageShell`: `px-page-x` (`clamp(16px, 3vw, 40px)`),
`py-page-y` (24px), and `gap-section` / `mb-section` (24px) between sections.

---

## 4. Elevation

`shadow-popover` for dropdowns, menus, tooltips and toasts, `shadow-modal` for
dialogs. That is the complete set. Inline content — cards, list rows, panels —
gets a border, not a shadow. Both tokens are theme-aware, so never pair them
with a `dark:` variant.

---

## 5. Motion

120–160ms, ease-out (`duration-150 ease-standard`), on **colour and opacity
only**. No scale, bounce, sliding or layout transitions — even the
indeterminate `ProgressBar` is an opacity pulse. `prefers-reduced-motion` is
honoured globally in `app.css`.

---

## 6. Focus

One treatment for the whole app, set once in `app.css`:

```css
:focus-visible {
  outline: 2px solid var(--focus);
  outline-offset: 2px;
}
```

Form controls additionally take a `border-focus` border and a translucent
`ring-focus/25`. Never remove a focus indicator without replacing it.

---

## 7. Primitives

From `$lib/components/ui`:

`Alert` · `Badge` · `BrandMark` · `Button` · `Card` · `ConfirmDialog` ·
`Dialog` · `EmptyState` · `Field` · `HighlightedSnippet` · `IconButton` ·
`Input` · `Kbd` · `ListRow` · `ListSkeleton` · `Markdown` · `Menu` ·
`PageHeader` · `PageShell` · `ProgressBar` · `SectionHeader` ·
`SegmentedControl` · `Select` · `SettingsRow` · `Skeleton` · `Spinner` ·
`StatusIndicator` · `Tabs` · `Textarea` · `ThinkingIndicator` · `Tooltip`

Plus `$lib/format` for dates, times and durations.

```svelte
<script lang="ts">
  import { Button, Menu, PageShell, SectionHeader, ListRow } from '$lib/components/ui';
  import { fmt } from '$lib/format';
</script>

<PageShell title={$t('patients.title')}>
  {#snippet actions()}
    <Button variant="primary" href="/patients/new">{$t('patients.new')}</Button>
  {/snippet}

  <SectionHeader title={$t('patients.recent')} />
  {#each patients as p (p.id)}
    <ListRow title={name(p)} secondary={$fmt.date(p.date_of_birth)} href="/patients/{p.id}">
      {#snippet trailing()}<Menu items={actionsFor(p)} size="sm" />{/snippet}
    </ListRow>
  {/each}
</PageShell>
```

### Usage rules

1. **Every page renders in a `PageShell`.** It owns padding, max-width
   (`width="wide|form|prose|full"`) and the left edge, and renders the page's
   only `<h1>`. Don't add `p-8` or `max-w-*` wrappers around it.
2. **One primary per view.** `Button variant="primary"` (ink) appears at most
   once per view. Everything else is `secondary` or `ghost`.
3. **Danger is an outline until confirmed.** `variant="danger"` is outlined;
   `variant="danger-solid"` exists only for `ConfirmDialog`. Destructive menu
   items use `MenuItem.danger` and are separated automatically.
4. **`ConfirmDialog`, never `window.confirm()`.** Name the record in the
   message ("Delete the diagnosis F33.1?"). Typed confirmations (RESET,
   export words) stay as they are inside the dialog body.
5. **`StatusIndicator` for any state.** Icon + text label, never a bare dot;
   interactive ones get a ≥24px target.
6. **No hover-only actions.** Row actions are visible or in a `⋯` `Menu`.
7. **Model and user text goes through `Markdown` or `HighlightedSnippet`.**
   Neither uses `{@html}`; raw HTML renders as text, only http(s)/mailto links
   are live, images never load. Don't add new `{@html}`.
8. **One date format.** `$fmt.date(x)` → `17.11.2026` in both languages;
   never call `toLocaleDateString('de-CH')` in a view.
9. **Settings rows use `SettingsRow`**: label and description left, control
   right, stacking automatically below 560px of row width.
10. **Tabs never widen the page.** `Tabs` scrolls inside its own box and
    contains its intrinsic width; route tabs pass `href` on every item.

### Variant and size names

| Primitive          | Variants / tones                                                                        | Sizes                       |
| ------------------ | --------------------------------------------------------------------------------------- | --------------------------- |
| `Button`           | `primary` (ink) · `secondary` (default) · `ghost` · `danger` (outline) · `danger-solid` | `sm` 28 · `md` 32 · `lg` 36 |
| `IconButton`       | tone `default` · `danger`                                                               | `sm` · `md` · `lg` (square) |
| `Badge`            | tone `neutral` · `info` · `success` · `warning` · `danger` (outlined, subtle)           | —                           |
| `StatusIndicator`  | tone `ok` · `warn` · `error` · `idle` · `busy`                                          | —                           |
| `ProgressBar`      | tone `default` · `danger`                                                               | `sm` · `md`                 |
| `SegmentedControl` | —                                                                                       | `sm` · `md`                 |
| `Tabs`             | —                                                                                       | `sm` · `md`                 |
| `Menu`             | trigger `ghost` (⋯) · `secondary` (labelled)                                            | `sm` · `md`                 |
| `PageShell`        | width `wide` (default) · `form` · `prose` · `full`                                      | —                           |
| `BrandMark`        | tone `brand` · `current`                                                                | 16 · 20 · 24 · 32 · 64      |

`Button` and `IconButton` render an `<a>` when given `href`. An anchor is never
`:disabled`, so Tailwind's `disabled:` variants do not apply to that branch —
both components instead attach `pointer-events-none opacity-50` directly, plus
`aria-disabled` and `tabindex="-1"`. The `href` is deliberately kept so the
element keeps its link role for assistive tech. If you hand-roll a disabled
link anywhere, do the same.

`Button` labels never wrap (`whitespace-nowrap shrink-0`). If a button
overflows, the parent layout is wrong — give the row `flex-wrap` or use
`SettingsRow`.

Note: `Select` renders a native `<select>`; pass an initial `value` that matches
one of the options, or it will show blank rather than defaulting to the first.

### The brand mark

`BrandMark` draws the glyph from `BrandMarkGlyph.svelte`, which currently holds
a **placeholder** (D7). To ship the real mark, replace the `<svg>` in
`BrandMarkGlyph.svelte` with the product owner's vector — keeping
`width/height="100%"`, `currentColor`, and `aria-hidden` — and nothing else
changes. Do not trace the PNG icon. Replace `dokassist/app-icon.svg` only once
the real vector exists.

---

## 8. Not the AI app

RamDoc uses local language models, and that should feel like an ordinary
capability of the tool rather than its personality. No robot or brain
iconography, no sparkles, no purple "AI" buttons, no emoji in UI chrome.
Generated content is marked with a plain label and, where it helps, a subtle
surface tint — the same vocabulary as everything else.

---

## 9. Adding to the system

1. Reach for an existing token. Almost always there is one.
2. If genuinely missing, add it to **both** `:root` and `.dark` in `app.css`,
   map it under `@theme inline`, and document it here.
3. If a pattern appears three times, it belongs in `src/lib/components/ui/`.
4. Run `pnpm check:design` and `pnpm check:i18n`.

Exceptions to the no-raw-colour rule live in the `ALLOW` list in
`scripts/check-design-tokens.mjs` and each needs a stated reason. There are
currently none: the modal scrim, the former exception, is now the `scrim`
token.

`pnpm check:i18n` also rejects string literals that a `{…}` text expression can
output (`{n === 1 ? 'measurement' : 'measurements'}`, `{name || 'Unknown'}`).
Literals passed to calls (`$t('a.b')`, `.join(', ')`), compared
(`status === 'active'`), used as an index, or shaped like a translation key
are ignored; untranslatable values go in its `ALLOW` list with a reason.

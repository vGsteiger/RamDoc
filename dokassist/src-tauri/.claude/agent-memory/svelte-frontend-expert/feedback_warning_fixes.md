---
name: Svelte 5 Build Warning Fix Patterns
description: Established patterns for resolving the categories of Svelte build warnings that appeared in this codebase (2026-03-21 cleanup)
type: feedback
---

Patterns confirmed for fixing Svelte 5 build warnings in this codebase:

**slot_element_deprecated**: Replace `<slot />` with `{@render children()}`. Layout files need `import type { Snippet } from 'svelte'` and `let { children }: { children: Snippet } = $props()` added to the script block.

**element_invalid_self_closing_tag**: `<textarea ... />` → `<textarea ...></textarea>`, `<div ... />` → `<div ...></div>`. Only void elements (input, br, hr, img, etc.) may self-close.

**a11y_label_has_associated_control**: Two cases:
- Display-only labels (no input): replace `<label>` with `<span>` or `<p>`.
- Labels paired with inputs: add matching `for="id"` to label and `id="..."` to the input.

**a11y_click_events_have_key_events + a11y_no_static_element_interactions** on modal overlays:
- Backdrop div: add `role="presentation"` and `onkeydown` (Escape handler).
- Dialog panel div: add `role="dialog"`, `aria-modal="true"`, `aria-labelledby` pointing to dialog title `id`.
- Drop zones: add `role="region"` with `aria-label`.

**a11y_autofocus**: Remove `autofocus` attribute. Instead: add `let el = $state<HTMLInputElement | null>(null)`, bind with `bind:this={el}`, and focus via `$effect(() => { if (condition && el) el.focus(); })`.

**state_referenced_locally**: Props used to initialise `$state(...)` won't track prop changes. Fix: keep the `$state` initialisation for mount-time value, then add a `$effect(() => { if (prop) { stateVar = ...; } })` that resets the state when the prop changes.

**Why:** These were all build warnings surfaced by `pnpm build` in the Svelte 5 compiler; resolving them keeps the build clean and ensures correct reactivity when prop values change.

**How to apply:** Apply these patterns whenever any of these warning codes appear in future `pnpm build` output.

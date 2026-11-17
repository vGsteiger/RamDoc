<!--
  PLACEHOLDER — replace with product-owner SVG (D7).

  This file is the ONLY place the brand glyph is drawn. To swap in the real
  mark, replace the <svg> below with the product owner's vector, keeping:
    - width="100%" height="100%" so BrandMark controls the size
    - stroke="currentColor" / fill="currentColor" (no hardcoded colour), so
      BrandMark can tint it with the brand token in both themes
    - aria-hidden="true" and focusable="false" (BrandMark owns the a11y name)
  Nothing else in the app needs to change. Do NOT trace the PNG icon, and
  replace dokassist/app-icon.svg only once the real vector exists.

  The placeholder is deliberately generic: one uniform stroke on a 24-unit
  grid, echoing "a single thin line" without imitating the ram glyph.
-->
<svg
  width="100%"
  height="100%"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
  data-brand-placeholder
>
  <path d="M7 19V9.5a5 5 0 0 1 10 0 3 3 0 0 1-6 0" />
</svg>

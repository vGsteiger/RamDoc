<script lang="ts">
  import BrandMarkGlyph from './BrandMarkGlyph.svelte';

  /*
   * The RamDoc mark. The glyph itself lives in BrandMarkGlyph.svelte (a
   * placeholder until the product owner's SVG arrives — D7); this wrapper owns
   * size, colour and accessibility, so swapping the glyph is a one-file change.
   *
   * Colour: `brand` (default) tints it with the logo red token — this and the
   * active-nav bar are the only places the brand red appears. `current`
   * inherits the surrounding text colour (e.g. monochrome contexts).
   */
  let {
    size = 20,
    tone = 'brand',
    title = undefined,
    class: className = '',
  }: {
    /** Rendered square size in px: 16, 20, 24, 32 or 64. */
    size?: 16 | 20 | 24 | 32 | 64;
    tone?: 'brand' | 'current';
    /** Accessible name. Omit when a visible wordmark sits next to it. */
    title?: string;
    class?: string;
  } = $props();
</script>

<span
  class="inline-flex shrink-0 {tone === 'brand' ? 'text-brand' : ''} {className}"
  style:width="{size}px"
  style:height="{size}px"
  role={title ? 'img' : undefined}
  aria-label={title}
  aria-hidden={title ? undefined : 'true'}
  data-brand-mark
>
  <BrandMarkGlyph />
</span>

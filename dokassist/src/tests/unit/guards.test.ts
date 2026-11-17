import { describe, it, expect } from 'vitest';
import { lineViolations } from '../../../scripts/check-design-tokens.mjs';
import { textExpressionLiterals } from '../../../scripts/check-i18n.mjs';

const labels = (line: string) => (lineViolations(line) as [string, string][]).map(([, m]) => m);

describe('check-design-tokens', () => {
  it.each([
    ['<h3 class="text-md font-semibold">', 'text-md'],
    ['<p class="text-sm text-fg">', 'text-sm'],
    ['<p class="md:text-2xl">', 'md:text-2xl'],
    ['<div class="fixed inset-0 bg-black/50">', 'bg-black/50'],
    ['<div class="bg-white/80">', 'bg-white/80'],
    ['<b class="font-bold">', 'font-bold'],
    ['<b class="hover:font-extrabold">', 'hover:font-extrabold'],
  ])('rejects %s', (line, match) => {
    expect(labels(line)).toContain(match);
  });

  it.each([
    '<h3 class="text-heading font-semibold text-fg">',
    '<div class="fixed inset-0 bg-scrim">',
    '<p class="text-caption text-fg-muted">',
    '<span class="text-[0.9em] font-medium">',
    '  // never write text-md or bg-black/50',
  ])('accepts %s', (line) => {
    expect(lineViolations(line)).toEqual([]);
  });
});

describe('check-i18n text expressions', () => {
  const values = (markup: string) =>
    (textExpressionLiterals(markup) as { value: string }[]).map((f) => f.value);

  it('flags literals a text expression can output', () => {
    expect(values(`<span>({n} {n === 1 ? 'measurement' : 'measurements'})</span>`)).toEqual([
      'measurement',
      'measurements',
    ]);
    expect(values(`<p>{name || 'Unbekannt'}</p>`)).toEqual(['Unbekannt']);
    expect(values('<p>{`${n} Sitzungen`}</p>')).toEqual(['  Sitzungen']);
  });

  it('ignores calls, comparisons, keys, attributes and block tags', () => {
    expect(values(`<p>{$t('common.save')}</p>`)).toEqual([]);
    expect(values(`<p>{items.join(', ')}</p>`)).toEqual([]);
    expect(values(`<p>{$t(ok ? 'a.yes' : 'a.no')}</p>`)).toEqual([]);
    expect(values(`<p>{status === 'active' ? x : y}</p>`)).toEqual([]);
    expect(values(`<div class={open ? 'block text-fg' : 'hidden'}>x</div>`)).toEqual([]);
    expect(values(`{#if kind === 'Report draft'}<p>x</p>{/if}`)).toEqual([]);
    expect(values(`{@const label = 'whatever'}`)).toEqual([]);
  });
});

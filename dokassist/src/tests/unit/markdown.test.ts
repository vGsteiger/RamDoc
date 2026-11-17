import { describe, it, expect } from 'vitest';
import { decodeEntities, parseMarkdown, safeHref } from '$lib/components/ui/markdown';
import {
  findMatchRanges,
  parseMarkedSnippet,
  segmentsFromRanges,
} from '$lib/components/ui/highlight';

describe('safeHref', () => {
  it('keeps absolute http(s) and mailto links', () => {
    expect(safeHref('https://example.ch/a?b=1')).toBe('https://example.ch/a?b=1');
    expect(safeHref('http://example.ch')).toBe('http://example.ch/');
    expect(safeHref('mailto:praxis@example.ch')).toBe('mailto:praxis@example.ch');
  });

  it.each([
    'javascript:alert(1)',
    'JaVaScRiPt:alert(1)',
    ' javascript:alert(1)',
    'java\tscript:alert(1)',
    'java\u0000script:alert(1)',
    'data:text/html;base64,PHNjcmlwdD4=',
    'vbscript:msgbox(1)',
    'file:///etc/passwd',
    '/patients/p1',
    '//evil.example',
    '',
  ])('rejects %j', (href) => {
    expect(safeHref(href)).toBeNull();
  });
});

describe('decodeEntities', () => {
  it('decodes named and numeric references to plain characters', () => {
    expect(decodeEntities('a &amp; b &lt;c&gt; &#228; &#xE4;')).toBe('a & b <c> ä ä');
  });

  it('leaves unknown and invalid references alone', () => {
    expect(decodeEntities('&bogus; &#xD800; &#0;')).toBe('&bogus; &#xD800; &#0;');
  });
});

describe('parseMarkdown', () => {
  it('maps the common chat constructs', () => {
    const tree = parseMarkdown('## Verlauf\n\n- **Schlaf**: besser\n- Antrieb\n\n1. eins');
    expect(tree.map((b) => b.t)).toEqual(['h', 'list', 'list']);
    const list = tree[1];
    expect(list.t === 'list' && list.ordered).toBe(false);
  });

  it('turns raw HTML into text nodes, never elements', () => {
    const tree = parseMarkdown('<script>alert(1)</script>\n\nHi <img src=x onerror=alert(1)>');
    const json = JSON.stringify(tree);
    expect(json).toContain('<script>alert(1)</script>');
    expect(json).toContain('<img src=x onerror=alert(1)>');
    // Every node is one of the closed set of display types.
    const types = new Set<string>();
    const walk = (n: unknown): void => {
      if (Array.isArray(n)) n.forEach(walk);
      else if (n && typeof n === 'object') {
        const node = n as Record<string, unknown>;
        if (typeof node.t === 'string') types.add(node.t);
        Object.values(node).forEach(walk);
      }
    };
    walk(tree);
    for (const t of types) {
      expect([
        'p',
        'text',
        'h',
        'code',
        'hr',
        'quote',
        'list',
        'table',
        'strong',
        'em',
        'del',
        'br',
        'link',
      ]).toContain(t);
    }
  });

  it('drops unsafe link targets but keeps the link text', () => {
    const [p] = parseMarkdown('[click](javascript:alert(1)) and [ok](https://ok.example)');
    expect(p.t).toBe('p');
    const links = p.t === 'p' ? p.children.filter((c) => c.t === 'link') : [];
    expect(links).toHaveLength(2);
    expect(links[0].t === 'link' && links[0].href).toBeNull();
    expect(links[1].t === 'link' && links[1].href).toBe('https://ok.example/');
  });

  it('never keeps an image source', () => {
    const json = JSON.stringify(parseMarkdown('![scan](https://tracker.example/pixel.png)'));
    expect(json).not.toContain('tracker.example');
    expect(json).toContain('scan');
  });

  it('keeps code literal', () => {
    const [p] = parseMarkdown('`&lt;b&gt;`');
    expect(p.t === 'p' && p.children[0]).toEqual({ t: 'code', text: '&lt;b&gt;' });
  });

  it('turns single newlines into breaks by default', () => {
    const [p] = parseMarkdown('a\nb');
    expect(p.t === 'p' && p.children.some((c) => c.t === 'br')).toBe(true);
    const [strict] = parseMarkdown('a\nb', false);
    expect(strict.t === 'p' && strict.children.some((c) => c.t === 'br')).toBe(false);
  });

  it('handles empty input', () => {
    expect(parseMarkdown('')).toEqual([]);
    expect(parseMarkdown(null)).toEqual([]);
  });
});

describe('parseMarkedSnippet', () => {
  it('splits FTS5 <mark> tags into matched segments, keeping the word', () => {
    expect(parseMarkedSnippet('Exploration der <mark>Belastung</mark>situation')).toEqual([
      { text: 'Exploration der ', match: false },
      { text: 'Belastung', match: true },
      { text: 'situation', match: false },
    ]);
  });

  it('keeps every other tag as literal text', () => {
    const segs = parseMarkedSnippet('<img src=x onerror=alert(1)> <mark><b>x</b></mark>');
    expect(segs).toEqual([
      { text: '<img src=x onerror=alert(1)> ', match: false },
      { text: '<b>x</b>', match: true },
    ]);
  });

  it('tolerates unbalanced tags', () => {
    expect(parseMarkedSnippet('a</mark>b<mark>c')).toEqual([
      { text: 'ab', match: false },
      { text: 'c', match: true },
    ]);
  });
});

describe('segmentsFromRanges and findMatchRanges', () => {
  it('merges overlapping, unsorted, out-of-bounds ranges', () => {
    expect(
      segmentsFromRanges('Anna Müller', [
        [5, 8],
        [0, 2],
        [7, 50],
      ])
    ).toEqual([
      { text: 'An', match: true },
      { text: 'na ', match: false },
      { text: 'Müller', match: true },
    ]);
  });

  it('finds case-insensitive term matches', () => {
    expect(findMatchRanges('Anna Müller-Anna', 'anna')).toEqual([
      [0, 4],
      [12, 16],
    ]);
    expect(findMatchRanges('x', '  ')).toEqual([]);
  });
});

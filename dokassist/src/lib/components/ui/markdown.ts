/**
 * Safe markdown → a small display tree, for Markdown.svelte.
 *
 * `marked` only tokenises here; it never produces HTML. The tokens are mapped
 * into the closed set of node types below and the component renders each one
 * with Svelte elements and text nodes, so there is no {@html} anywhere:
 *
 *   - raw HTML in the source (block or inline) becomes literal text
 *   - link targets are kept only for http(s) and mailto; anything else
 *     (javascript:, data:, vbscript:, file:, relative paths) renders as plain text
 *   - images never load (local-first, no network); their alt text is shown
 *
 * Model output is untrusted input: a prompt-injected document can ask the
 * model to emit markup. This module is the line that keeps it text.
 */
import { Marked, type Token, type Tokens } from 'marked';

export type MdInline =
  | { t: 'text'; text: string }
  | { t: 'strong' | 'em' | 'del'; children: MdInline[] }
  | { t: 'code'; text: string }
  | { t: 'br' }
  | { t: 'link'; href: string | null; children: MdInline[] };

export type MdAlign = 'left' | 'center' | 'right' | null;

export type MdBlock =
  | { t: 'p'; children: MdInline[] }
  | { t: 'text'; children: MdInline[] }
  | { t: 'h'; depth: number; children: MdInline[] }
  | { t: 'code'; text: string; lang: string }
  | { t: 'hr' }
  | { t: 'quote'; children: MdBlock[] }
  | {
      t: 'list';
      ordered: boolean;
      start: number | null;
      items: { checked: boolean | null; children: MdBlock[] }[];
    }
  | { t: 'table'; align: MdAlign[]; header: MdInline[][]; rows: MdInline[][][] };

const SAFE_PROTOCOL = /^(?:https?:|mailto:)/i;

/**
 * Returns the href if it is an absolute http(s) or mailto URL, else null.
 * Control characters and whitespace are stripped first because browsers
 * ignore them inside a scheme (`java\tscript:` is still javascript:).
 */
export function safeHref(href: string | null | undefined): string | null {
  if (!href) return null;
  // eslint-disable-next-line no-control-regex
  const cleaned = href.replace(/[\u0000- \u007f-\u009f]/g, '');
  if (!SAFE_PROTOCOL.test(cleaned)) return null;
  try {
    const url = new URL(cleaned);
    return ['http:', 'https:', 'mailto:'].includes(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
}

const NAMED: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
  nbsp: ' ',
  shy: '­',
  ndash: '–',
  mdash: '—',
  hellip: '…',
};

/**
 * Decodes the character references CommonMark expects to see decoded. The
 * result is only ever used as a text node, so decoding `&lt;script&gt;`
 * yields the visible text "<script>", never an element.
 */
export function decodeEntities(text: string): string {
  return text.replace(/&(#x[0-9a-f]{1,6}|#\d{1,7}|[a-z]+);/gi, (match, body: string) => {
    if (body[0] === '#') {
      const code =
        body[1] === 'x' || body[1] === 'X' ? parseInt(body.slice(2), 16) : parseInt(body.slice(1));
      if (!Number.isFinite(code) || code <= 0 || code > 0x10ffff) return match;
      if (code >= 0xd800 && code <= 0xdfff) return match;
      return String.fromCodePoint(code);
    }
    return NAMED[body.toLowerCase()] ?? match;
  });
}

const BR_TAG = /^<br\s*\/?>$/i;

function inline(tokens: Token[] | undefined): MdInline[] {
  const out: MdInline[] = [];
  const pushText = (text: string) => {
    if (!text) return;
    const prev = out[out.length - 1];
    if (prev && prev.t === 'text') prev.text += text;
    else out.push({ t: 'text', text });
  };
  for (const token of tokens ?? []) {
    switch (token.type) {
      case 'text': {
        const tok = token as Tokens.Text;
        if (tok.tokens?.length) out.push(...inline(tok.tokens));
        else pushText(decodeEntities(tok.text));
        break;
      }
      case 'escape':
        pushText(decodeEntities((token as Tokens.Escape).text));
        break;
      case 'strong':
      case 'em':
      case 'del':
        out.push({
          t: token.type,
          children: inline((token as Tokens.Strong | Tokens.Em | Tokens.Del).tokens),
        });
        break;
      case 'codespan':
        // Code spans are literal: CommonMark does not decode entities in them.
        out.push({ t: 'code', text: (token as Tokens.Codespan).text });
        break;
      case 'br':
        out.push({ t: 'br' });
        break;
      case 'link': {
        const tok = token as Tokens.Link;
        out.push({ t: 'link', href: safeHref(tok.href), children: inline(tok.tokens) });
        break;
      }
      case 'image':
        pushText((token as Tokens.Image).text);
        break;
      case 'html': {
        const raw = (token as Tokens.HTML).raw;
        if (BR_TAG.test(raw.trim())) out.push({ t: 'br' });
        else pushText(raw);
        break;
      }
      default:
        pushText('raw' in token && typeof token.raw === 'string' ? token.raw : '');
    }
  }
  return out;
}

function blocks(tokens: Token[] | undefined): MdBlock[] {
  const out: MdBlock[] = [];
  for (const token of tokens ?? []) {
    switch (token.type) {
      case 'space':
      case 'def':
        break;
      case 'paragraph':
        out.push({ t: 'p', children: inline((token as Tokens.Paragraph).tokens) });
        break;
      case 'text': {
        // Block-level text appears inside tight list items.
        const tok = token as Tokens.Text;
        out.push({
          t: 'text',
          children: tok.tokens?.length
            ? inline(tok.tokens)
            : [{ t: 'text', text: decodeEntities(tok.text) }],
        });
        break;
      }
      case 'heading': {
        const tok = token as Tokens.Heading;
        out.push({ t: 'h', depth: tok.depth, children: inline(tok.tokens) });
        break;
      }
      case 'code': {
        const tok = token as Tokens.Code;
        out.push({ t: 'code', text: tok.text, lang: (tok.lang ?? '').split(/\s/)[0] });
        break;
      }
      case 'hr':
        out.push({ t: 'hr' });
        break;
      case 'blockquote':
        out.push({ t: 'quote', children: blocks((token as Tokens.Blockquote).tokens) });
        break;
      case 'list': {
        const tok = token as Tokens.List;
        const start = tok.ordered && tok.start !== '' ? Number(tok.start) : null;
        out.push({
          t: 'list',
          ordered: tok.ordered,
          start: Number.isFinite(start) ? start : null,
          items: tok.items.map((item) => ({
            checked: item.task ? Boolean(item.checked) : null,
            children: blocks(item.tokens.filter((child) => child.type !== 'checkbox')),
          })),
        });
        break;
      }
      case 'table': {
        const tok = token as Tokens.Table;
        out.push({
          t: 'table',
          align: tok.align.map((a) => a ?? null),
          header: tok.header.map((cell) => inline(cell.tokens)),
          rows: tok.rows.map((row) => row.map((cell) => inline(cell.tokens))),
        });
        break;
      }
      case 'html':
        // Raw HTML is shown as the text it is, never interpreted.
        out.push({ t: 'p', children: [{ t: 'text', text: (token as Tokens.HTML).raw.trim() }] });
        break;
      default:
        if ('raw' in token && typeof token.raw === 'string' && token.raw.trim()) {
          out.push({ t: 'p', children: [{ t: 'text', text: token.raw.trim() }] });
        }
    }
  }
  return out;
}

const parsers = {
  breaks: new Marked({ gfm: true, breaks: true }),
  strict: new Marked({ gfm: true, breaks: false }),
};

/**
 * Parses markdown into the safe display tree. `breaks` (default true) turns
 * single newlines into line breaks, which is how chat models and clinicians
 * write.
 */
export function parseMarkdown(source: string | null | undefined, breaks = true): MdBlock[] {
  if (!source) return [];
  return blocks((breaks ? parsers.breaks : parsers.strict).lexer(source));
}

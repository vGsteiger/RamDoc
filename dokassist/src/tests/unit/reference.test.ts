import { describe, it, expect } from 'vitest';
import { literatureRedirect, referenceHref, referenceTab } from '$lib/reference';

const url = (path: string) => new URL(path, 'http://localhost');
const search = (path: string) => url(path).search;

describe('literatureRedirect', () => {
  it('sends /literature to the Literature tab of Reference', () => {
    expect(literatureRedirect(search('/literature'))).toBe('/reference?tab=literature');
  });

  it('preserves the query string', () => {
    expect(literatureRedirect(search('/literature?q=Depression&x=1'))).toBe(
      '/reference?tab=literature&q=Depression&x=1'
    );
  });

  it('always lands on the literature tab, even with a stray tab parameter', () => {
    expect(literatureRedirect(search('/literature?tab=medications&q=a'))).toBe(
      '/reference?tab=literature&q=a'
    );
  });
});

describe('referenceTab and referenceHref', () => {
  it('defaults to medications', () => {
    expect(referenceTab(new URLSearchParams(''))).toBe('medications');
    expect(referenceTab(new URLSearchParams('tab=bogus'))).toBe('medications');
    expect(referenceTab(new URLSearchParams('tab=literature'))).toBe('literature');
  });

  it('builds tab links that round-trip', () => {
    expect(referenceHref('medications')).toBe('/reference');
    expect(referenceHref('literature')).toBe('/reference?tab=literature');
    expect(referenceHref('medications', 'Sertralin')).toBe('/reference?q=Sertralin');
    const href = referenceHref('literature', 'Depression');
    expect(referenceTab(url(href).searchParams)).toBe('literature');
  });
});

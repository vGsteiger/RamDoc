/**
 * The Reference section (D9): medication reference first, then the
 * literature library, as tabs of `/reference?tab=…`. The old `/literature`
 * route redirects here and keeps working.
 */
export type ReferenceTab = 'medications' | 'literature';

export const REFERENCE_PATH = '/reference';

/** The active tab for a `/reference` URL; medications unless asked otherwise. */
export function referenceTab(searchParams: { get(name: string): string | null }): ReferenceTab {
  return searchParams.get('tab') === 'literature' ? 'literature' : 'medications';
}

/** Href for a Reference tab, optionally carrying a search query. */
export function referenceHref(tab: ReferenceTab, query?: string): string {
  const params = new URLSearchParams();
  if (tab !== 'medications') params.set('tab', tab);
  if (query) params.set('q', query);
  const search = params.toString();
  return search ? `${REFERENCE_PATH}?${search}` : REFERENCE_PATH;
}

/**
 * Where `/literature` sends the user: the Literature tab, preserving every
 * query parameter of the old URL (an explicit `tab` is overridden). Takes the
 * search string, because a load function may not read `url.hash`.
 */
export function literatureRedirect(search: string): string {
  const params = new URLSearchParams(search);
  params.delete('tab');
  const rest = params.toString();
  return `${REFERENCE_PATH}?tab=literature${rest ? `&${rest}` : ''}`;
}

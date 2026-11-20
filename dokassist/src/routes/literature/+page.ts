import { redirect } from '@sveltejs/kit';
import { literatureRedirect } from '$lib/reference';
import type { PageLoad } from './$types';

// Literature now lives under Reference (D9). Old links and bookmarks keep
// working: the Literature tab opens with the original query preserved.
export const load: PageLoad = ({ url }) => {
  redirect(307, literatureRedirect(url.search));
};

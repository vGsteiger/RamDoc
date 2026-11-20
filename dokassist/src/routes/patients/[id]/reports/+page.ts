import { redirect } from '@sveltejs/kit';
import { documentsUrl } from '$lib/components/documents/documents';
import type { PageLoad } from './$types';

// The report list lives in the unified Documents view now; keep the old URL working.
export const load: PageLoad = ({ params }) => {
  redirect(307, documentsUrl(params.id, 'reports'));
};

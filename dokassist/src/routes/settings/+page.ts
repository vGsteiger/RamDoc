import { redirect } from '@sveltejs/kit';
import { DEFAULT_SETTINGS_PATH } from '$lib/components/settings/sections';

// /settings has no content of its own: the section list lives in the layout
// and every pane is a sub-route. Existing links to /settings keep working.
export function load() {
  redirect(307, DEFAULT_SETTINGS_PATH);
}

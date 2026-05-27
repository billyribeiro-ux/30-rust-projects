import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals, url }) => {
  if (!locals.user) {
    const next = url.pathname + url.search;
    redirect(303, `/login?next=${encodeURIComponent(next)}`);
  }
  return { user: locals.user };
};

import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

/**
 * Auth gate for every page in the (app) group. If hooks.server.ts didn't
 * find a valid user, bounce to /login. The original destination is
 * preserved as ?next=/path so the login page can return the user there.
 */
export const load: LayoutServerLoad = async ({ locals, url }) => {
  if (!locals.user) {
    const next = url.pathname + url.search;
    redirect(303, `/login?next=${encodeURIComponent(next)}`);
  }
  return { user: locals.user };
};

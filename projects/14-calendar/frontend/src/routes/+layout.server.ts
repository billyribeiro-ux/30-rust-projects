import type { LayoutServerLoad } from './$types';

/**
 * Root server load: surfaces `user` (from hooks.server.ts) to every page.
 * Children read it via `data.user` or `page.data.user`.
 */
export const load: LayoutServerLoad = async ({ locals }) => {
  return { user: locals.user };
};

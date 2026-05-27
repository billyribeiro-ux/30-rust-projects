import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

/**
 * If the user is already authenticated, kick them out of the (auth) area
 * straight to the dashboard. Login/register are useless for an authed user.
 */
export const load: LayoutServerLoad = async ({ locals }) => {
  if (locals.user) {
    redirect(303, '/');
  }
  return {};
};

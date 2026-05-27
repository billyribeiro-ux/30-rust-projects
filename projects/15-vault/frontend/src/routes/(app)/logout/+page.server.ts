import { redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3014';

export const load: PageServerLoad = async () => {
  redirect(303, '/vault');
};

export const actions: Actions = {
  default: async ({ fetch, cookies, locals }) => {
    if (locals.sessionCookie) {
      try {
        await fetch(`${BACKEND_URL}/api/auth/logout`, {
          method: 'POST',
          headers: { cookie: `${SESSION_COOKIE}=${locals.sessionCookie}` }
        });
      } catch {
        /* ignore */
      }
    }
    cookies.delete(SESSION_COOKIE, { path: '/' });
    redirect(303, '/login');
  }
};

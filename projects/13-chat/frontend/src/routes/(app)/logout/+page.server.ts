import { redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3012';

export const load: PageServerLoad = async () => {
  // No GET version — only via the topbar form.
  redirect(303, '/');
};

export const actions: Actions = {
  default: async ({ fetch, cookies, locals }) => {
    if (locals.sessionCookie) {
      // Best-effort: tell the backend to delete the session. Even if it
      // fails (e.g., backend down), we clear the cookie locally so the
      // user sees themselves as signed out.
      try {
        await fetch(`${BACKEND_URL}/api/auth/logout`, {
          method: 'POST',
          headers: { cookie: `${SESSION_COOKIE}=${locals.sessionCookie}` }
        });
      } catch {
        /* ignore — we still clear locally */
      }
    }
    cookies.delete(SESSION_COOKIE, { path: '/' });
    redirect(303, '/login');
  }
};

import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { kpisApi } from '$lib/api';

const SESSION_COOKIE = 'app_session';
function withCookie(cookie: string | null): typeof fetch {
  return (input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        ...(cookie ? { cookie: `${SESSION_COOKIE}=${cookie}` } : {})
      }
    });
}

export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.user) redirect(303, '/login');
  const kpis = await kpisApi.snapshot(withCookie(locals.sessionCookie));
  return { kpis };
};

import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { ApiCallError, interviewsApi } from '$lib/api';

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

export const load: PageServerLoad = async ({ params, locals }) => {
  if (!locals.user) redirect(303, '/login');
  try {
    const interview = await interviewsApi.get(withCookie(locals.sessionCookie), params.id);
    return { interview };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Interview not found.');
    throw err;
  }
};

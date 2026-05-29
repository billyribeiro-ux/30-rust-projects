import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { ApiCallError, coursesApi, enrollApi } from '$lib/api';

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
  try {
    const detail = await coursesApi.get(withCookie(locals.sessionCookie), params.slug);
    return { detail };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Course not found.');
    throw err;
  }
};

export const actions: Actions = {
  buy: async ({ params, locals }) => {
    if (!locals.user) redirect(303, `/login?next=/c/${params.slug}`);
    try {
      const out = await enrollApi.checkout(withCookie(locals.sessionCookie), params.slug);
      redirect(303, out.checkout_url);
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

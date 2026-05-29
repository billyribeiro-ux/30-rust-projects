import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
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

export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.user) redirect(303, '/login');
  return { interviews: await interviewsApi.list(withCookie(locals.sessionCookie)) };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const data = await request.formData();
    const email = String(data.get('candidate_email') ?? '').trim().toLowerCase();
    const language = String(data.get('language') ?? 'rust');
    if (!email) return fail(422, { error: 'Candidate email required.' });
    try {
      const out = await interviewsApi.create(withCookie(locals.sessionCookie), email, language);
      redirect(303, `/i/${out.id}`);
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { ApiCallError, keysApi, usageApi } from '$lib/api';

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
  const f = withCookie(locals.sessionCookie);
  const [keys, usage] = await Promise.all([keysApi.list(f), usageApi.summary(f)]);
  return { keys, usage };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const rpm = Number(data.get('rpm_limit') ?? 60);
    if (!name) return fail(422, { error: 'name required' });
    try {
      const out = await keysApi.create(withCookie(locals.sessionCookie), name, rpm);
      return { created: out };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  revoke: async ({ request, locals }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(422, { error: 'id required' });
    try {
      await keysApi.revoke(withCookie(locals.sessionCookie), id);
      return { revoked: id };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

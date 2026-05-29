import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { ApiCallError, tenantsApi } from '$lib/api';

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
  const tenants = await tenantsApi.list(f);
  return { tenants };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const data = await request.formData();
    const slug = String(data.get('slug') ?? '').trim().toLowerCase();
    const name = String(data.get('name') ?? '').trim();
    if (!slug || !name) return fail(422, { slug, name, error: 'Slug and name required.' });
    try {
      await tenantsApi.create(withCookie(locals.sessionCookie), slug, name);
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { slug, name, error: err.message });
      throw err;
    }
    return { ok: true };
  }
};

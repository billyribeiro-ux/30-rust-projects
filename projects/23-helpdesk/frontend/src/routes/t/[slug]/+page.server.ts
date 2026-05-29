import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { ApiCallError, ticketsApi } from '$lib/api';

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

export const load: PageServerLoad = async ({ locals, params, url }) => {
  if (!locals.user) redirect(303, '/login');
  const status = url.searchParams.get('status') ?? 'open';
  const f = withCookie(locals.sessionCookie);
  const tickets = await ticketsApi.list(f, params.slug, status);
  return { tickets, slug: params.slug, status };
};

export const actions: Actions = {
  create: async ({ request, params, locals }) => {
    const data = await request.formData();
    const subject = String(data.get('subject') ?? '').trim();
    const body = String(data.get('body') ?? '').trim();
    const priority = String(data.get('priority') ?? 'normal');
    if (!subject || !body) return fail(422, { error: 'Subject and body required.' });
    try {
      await ticketsApi.create(
        withCookie(locals.sessionCookie),
        params.slug,
        subject,
        body,
        priority
      );
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
    return { ok: true };
  }
};

import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { jobsApi, queuesApi, ApiCallError } from '$lib/api';

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

export const load: PageServerLoad = async ({ locals, url }) => {
  if (!locals.user) redirect(303, '/login');
  const queue = url.searchParams.get('queue') ?? 'default';
  const status = url.searchParams.get('status') ?? 'pending';
  const f = withCookie(locals.sessionCookie);
  try {
    const [queues, jobs] = await Promise.all([
      queuesApi.list(f),
      jobsApi.list(f, { queue, status, limit: 100 })
    ]);
    return { queues, jobs, queue, status };
  } catch (err) {
    if (err instanceof ApiCallError) error(err.status, err.message);
    throw err;
  }
};

export const actions: Actions = {
  enqueue: async ({ request, locals }) => {
    const data = await request.formData();
    const kind = String(data.get('kind') ?? '').trim();
    const payloadRaw = String(data.get('payload') ?? '{}');
    if (!kind) return fail(422, { error: 'Kind required.' });
    let payload: Record<string, unknown> = {};
    try {
      payload = JSON.parse(payloadRaw);
    } catch {
      return fail(422, { error: 'Payload must be valid JSON.' });
    }
    const f = withCookie(locals.sessionCookie);
    try {
      await jobsApi.enqueue(f, { kind, payload });
      return { ok: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  retry: async ({ request, locals }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(422, { error: 'id required.' });
    const f = withCookie(locals.sessionCookie);
    try {
      await jobsApi.retry(f, id);
      return { ok: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

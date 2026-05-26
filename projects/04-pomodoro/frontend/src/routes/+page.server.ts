import { fail } from '@sveltejs/kit';
import { sessionsApi, ApiCallError, type CreateSessionInput } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const [sessions, stats] = await Promise.all([sessionsApi.list(fetch, 30), sessionsApi.stats(fetch)]);
  return { sessions, stats };
};

export const actions: Actions = {
  record: async ({ request, fetch }) => {
    const data = await request.formData();
    const input: CreateSessionInput = {
      kind: String(data.get('kind') ?? '') as CreateSessionInput['kind'],
      label: data.get('label') ? String(data.get('label')) : null,
      planned_seconds: Number(data.get('planned_seconds') ?? 0),
      actual_seconds: Number(data.get('actual_seconds') ?? 0),
      started_at: String(data.get('started_at') ?? ''),
      ended_at: String(data.get('ended_at') ?? '')
    };

    try {
      await sessionsApi.create(fetch, input);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await sessionsApi.remove(fetch, id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

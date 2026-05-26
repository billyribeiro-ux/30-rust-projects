import { error, fail, redirect } from '@sveltejs/kit';
import { applicationsApi, eventsApi, nextStepsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { AppStatus } from '$lib/types';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  const f = serverFetch(locals.sessionCookie);
  try {
    const [app, events, nextSteps] = await Promise.all([
      applicationsApi.get(f, params.id),
      eventsApi.list(f, params.id),
      nextStepsApi.list(f, params.id)
    ]);
    return { app, events, nextSteps };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Application not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  updateStatus: async ({ params, request, locals }) => {
    const status = String((await request.formData()).get('status') ?? '') as AppStatus;
    if (!status) return fail(422, { error: 'Missing status.' });
    try {
      await applicationsApi.update(serverFetch(locals.sessionCookie), params.id, { status });
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  addNote: async ({ params, request, locals }) => {
    const body = String((await request.formData()).get('body') ?? '').trim();
    if (!body) return fail(422, { error: 'Note body is required.' });
    try {
      await eventsApi.createNote(serverFetch(locals.sessionCookie), params.id, body);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  addStep: async ({ params, request, locals }) => {
    const data = await request.formData();
    const body = String(data.get('body') ?? '').trim();
    const due_at = String(data.get('due_at') ?? '');
    if (!body || !due_at) return fail(422, { error: 'Body and due date required.' });
    try {
      // datetime-local emits "YYYY-MM-DDTHH:mm"; convert to ISO with seconds + Z
      const iso = new Date(due_at).toISOString();
      await nextStepsApi.create(serverFetch(locals.sessionCookie), params.id, body, iso);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  completeStep: async ({ params, request, locals }) => {
    const stepId = String((await request.formData()).get('step_id') ?? '');
    if (!stepId) return fail(422, { error: 'Missing step id.' });
    try {
      await nextStepsApi.complete(serverFetch(locals.sessionCookie), params.id, stepId);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },
  remove: async ({ params, locals }) => {
    await applicationsApi.remove(serverFetch(locals.sessionCookie), params.id);
    redirect(303, '/applications');
  }
};

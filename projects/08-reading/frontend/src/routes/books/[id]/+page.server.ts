import { error, redirect } from '@sveltejs/kit';
import {
  booksApi,
  sessionsApi,
  highlightsApi,
  ApiCallError
} from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const [book, sessions, highlights] = await Promise.all([
      booksApi.get(fetch, params.id),
      sessionsApi.list(fetch, params.id),
      highlightsApi.list(fetch, params.id)
    ]);
    return { book, sessions, highlights };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Book not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  updateBook: async ({ params, request, fetch }) => {
    const data = await request.formData();
    const payload = JSON.parse(String(data.get('payload') ?? '{}'));
    try {
      await booksApi.update(fetch, params.id, payload);
      return { success: true, kind: 'updateBook' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { success: false, error: err.message };
      }
      throw err;
    }
  },

  addSession: async ({ params, request, fetch }) => {
    const data = await request.formData();
    const pages_read = Number(data.get('pages_read') ?? 0);
    const duration_minutes = Number(data.get('duration_minutes') ?? 0);
    try {
      await sessionsApi.create(fetch, params.id, { pages_read, duration_minutes });
      return { success: true, kind: 'addSession' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { success: false, error: err.message };
      }
      throw err;
    }
  },

  removeSession: async ({ params, request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    try {
      await sessionsApi.remove(fetch, params.id, id);
      return { success: true, kind: 'removeSession' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { success: false, error: err.message };
      }
      throw err;
    }
  },

  addHighlight: async ({ params, request, fetch }) => {
    const data = await request.formData();
    const quote = String(data.get('quote') ?? '');
    const note = String(data.get('note') ?? '');
    const pageRaw = String(data.get('page') ?? '');
    const page = pageRaw ? Number(pageRaw) : null;
    try {
      await highlightsApi.create(fetch, params.id, { quote, note, page });
      return { success: true, kind: 'addHighlight' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { success: false, error: err.message };
      }
      throw err;
    }
  },

  removeHighlight: async ({ params, request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    try {
      await highlightsApi.remove(fetch, params.id, id);
      return { success: true, kind: 'removeHighlight' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { success: false, error: err.message };
      }
      throw err;
    }
  },

  removeBook: async ({ params, fetch }) => {
    try {
      await booksApi.remove(fetch, params.id);
    } catch (err) {
      if (!(err instanceof ApiCallError) || err.status !== 404) throw err;
    }
    redirect(303, '/');
  }
};

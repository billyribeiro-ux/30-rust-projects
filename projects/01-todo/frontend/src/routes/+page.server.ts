import { fail } from '@sveltejs/kit';
import { todosApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const todos = await todosApi.list(fetch);
  return { todos };
};

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const data = await request.formData();
    const title = String(data.get('title') ?? '').trim();

    if (!title) {
      return fail(422, { title, error: 'Title must not be empty.' });
    }
    if (title.length > 200) {
      return fail(422, { title, error: 'Title must be 200 characters or fewer.' });
    }

    try {
      await todosApi.create(fetch, title);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { title, error: err.message });
      }
      throw err;
    }
  },

  toggle: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    const done = data.get('done') === 'true';

    if (!id) return fail(422, { error: 'Missing id.' });

    try {
      await todosApi.update(fetch, id, { done });
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { error: err.message });
      }
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');

    if (!id) return fail(422, { error: 'Missing id.' });

    try {
      await todosApi.remove(fetch, id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { error: err.message });
      }
      throw err;
    }
  }
};

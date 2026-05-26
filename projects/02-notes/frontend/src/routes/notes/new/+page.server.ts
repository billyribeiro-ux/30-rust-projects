import { fail, redirect } from '@sveltejs/kit';
import { notesApi, ApiCallError } from '$lib/api';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const data = await request.formData();
    const title = String(data.get('title') ?? '').trim();
    const body_md = String(data.get('body_md') ?? '');

    if (!title) {
      return fail(422, { title, body_md, error: 'Title must not be empty.' });
    }
    if (title.length > 200) {
      return fail(422, { title, body_md, error: 'Title must be 200 characters or fewer.' });
    }

    try {
      const note = await notesApi.create(fetch, title, body_md);
      redirect(303, `/notes/${note.slug}`);
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { title, body_md, error: err.message });
      }
      throw err;
    }
  }
};

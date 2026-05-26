import { fail } from '@sveltejs/kit';
import { habitsApi, ApiCallError } from '$lib/api';
import { ALLOWED_COLORS } from '$lib/types';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const habits = await habitsApi.list(fetch);
  return { habits };
};

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const color = String(data.get('color') ?? '').toUpperCase();

    if (!name) return fail(422, { name, color, error: 'Name must not be empty.' });
    if (name.length > 80) return fail(422, { name, color, error: 'Name must be 80 chars or fewer.' });
    if (!(ALLOWED_COLORS as readonly string[]).includes(color)) {
      return fail(422, { name, color, error: 'Choose a color from the palette.' });
    }

    try {
      await habitsApi.create(fetch, name, color);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { name, color, error: err.message });
      throw err;
    }
  },

  toggle: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    const date = String(data.get('date') ?? '');
    if (!id || !date) return fail(422, { error: 'Missing id or date.' });

    try {
      await habitsApi.toggle(fetch, id, date);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });

    try {
      await habitsApi.remove(fetch, id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

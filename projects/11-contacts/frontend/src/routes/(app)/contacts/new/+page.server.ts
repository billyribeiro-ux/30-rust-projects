import { fail, redirect } from '@sveltejs/kit';
import { contactsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, locals }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const email = String(data.get('email') ?? '').trim();
    const phone = String(data.get('phone') ?? '').trim();
    const company = String(data.get('company') ?? '').trim();
    const notes = String(data.get('notes') ?? '').trim();
    const tagsRaw = String(data.get('tags') ?? '');
    const tags = tagsRaw
      .split(/[,\s]+/)
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean);

    if (!name) return fail(422, { name, email, phone, company, notes, tagsRaw, error: 'Name is required.' });

    try {
      const created = await contactsApi.create(serverFetch(locals.sessionCookie), {
        name,
        email,
        phone,
        company,
        notes,
        tags
      });
      redirect(303, `/contacts/${created.id}`);
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { name, email, phone, company, notes, tagsRaw, error: err.message });
      }
      throw err;
    }
  }
};

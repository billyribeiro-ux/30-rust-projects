import { fail } from '@sveltejs/kit';
import { contactsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
  const q = url.searchParams.get('q') ?? '';
  const tag = url.searchParams.get('tag') ?? '';
  const f = serverFetch(locals.sessionCookie);
  const contacts = await contactsApi.list(f, {
    q: q || undefined,
    tag: tag || undefined
  });
  return { contacts, q, tag };
};

export const actions: Actions = {
  remove: async ({ request, locals }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await contactsApi.remove(serverFetch(locals.sessionCookie), id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

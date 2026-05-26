import { error, redirect } from '@sveltejs/kit';
import { contactsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  const f = serverFetch(locals.sessionCookie);
  try {
    const contact = await contactsApi.get(f, params.id);
    return { contact };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Contact not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  touch: async ({ params, locals }) => {
    await contactsApi.touch(serverFetch(locals.sessionCookie), params.id);
    return { success: true };
  },
  remove: async ({ params, locals }) => {
    await contactsApi.remove(serverFetch(locals.sessionCookie), params.id);
    redirect(303, '/contacts');
  }
};

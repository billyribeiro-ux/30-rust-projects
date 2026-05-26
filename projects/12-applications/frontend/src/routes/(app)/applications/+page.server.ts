import { applicationsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { AppStatus } from '$lib/types';

export const load: PageServerLoad = async ({ locals, url }) => {
  const statusParam = url.searchParams.get('status') as AppStatus | null;
  const apps = await applicationsApi.list(serverFetch(locals.sessionCookie), {
    status: statusParam ?? undefined
  });
  return { apps, status: statusParam };
};

export const actions: Actions = {
  remove: async ({ request, locals }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await applicationsApi.remove(serverFetch(locals.sessionCookie), id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

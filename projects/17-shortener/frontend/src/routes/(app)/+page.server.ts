import { ApiCallError, linksApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  const f = serverFetch(locals.sessionCookie);
  const links = await linksApi.list(f);
  return {
    links,
    publicBase:
      // The redirect goes through the BACKEND (project port 3015), not the
      // frontend — / + slug is a backend route. We expose the env value so
      // the UI shows the right "Copy" URL.
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (process.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3015'
  };
};

export const actions: Actions = {
  newLink: async ({ request, locals }) => {
    const form = await request.formData();
    const target_url = (form.get('target_url') ?? '').toString();
    const slug = (form.get('slug') ?? '').toString().trim() || undefined;
    try {
      const link = await linksApi.create(serverFetch(locals.sessionCookie), target_url, slug);
      return { ok: true, link };
    } catch (e) {
      if (e instanceof ApiCallError)
        return fail(e.status, { error: e.message, target_url, slug });
      throw e;
    }
  }
};

import { ApiCallError, pollsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, locals }) => {
    const form = await request.formData();
    const slug = (form.get('slug') ?? '').toString().trim();
    const question = (form.get('question') ?? '').toString().trim();
    // option_N inputs — keep insertion order
    const options: string[] = [];
    for (const [k, v] of form.entries()) {
      if (k.startsWith('option_')) {
        const s = v.toString().trim();
        if (s) options.push(s);
      }
    }

    try {
      const poll = await pollsApi.create(serverFetch(locals.sessionCookie), {
        slug,
        question,
        options
      });
      redirect(303, `/polls/${poll.slug}`);
    } catch (e) {
      if (e instanceof ApiCallError) {
        return fail(e.status, { error: e.message, slug, question, options });
      }
      throw e;
    }
  }
};

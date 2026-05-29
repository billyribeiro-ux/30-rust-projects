import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { postsApi, subscribeApi, ApiCallError } from '$lib/api';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    const posts = await postsApi.list(fetch);
    return { posts };
  } catch (err) {
    if (err instanceof ApiCallError) return { posts: [], error: err.message };
    return { posts: [], error: 'Could not load posts.' };
  }
};

export const actions: Actions = {
  subscribe: async ({ request, fetch }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    const plan = (String(data.get('plan') ?? 'free') as 'free' | 'pro');
    if (!email) return fail(422, { error: 'Email required.' });
    try {
      const out = await subscribeApi.start(fetch, email, plan);
      if (out.kind === 'pro') {
        redirect(303, out.checkout_url);
      }
      // free: also send a magic link so they can sign in immediately.
      await subscribeApi.magicStart(fetch, email).catch(() => null);
      return { message: 'Subscribed. Check your inbox for a sign-in link.' };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};

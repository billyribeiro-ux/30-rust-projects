import { ApiCallError, roomsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  const rooms = await roomsApi.list(serverFetch(locals.sessionCookie));
  return { rooms };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = await request.formData();
    const slug = (form.get('slug') ?? '').toString();
    const name = (form.get('name') ?? '').toString();
    let createdSlug: string;
    try {
      const room = await roomsApi.create(serverFetch(locals.sessionCookie), slug, name);
      createdSlug = room.slug;
    } catch (e) {
      if (e instanceof ApiCallError) return fail(e.status, { error: e.message, slug, name });
      throw e;
    }
    throw redirect(303, `/rooms/${createdSlug}`);
  },
  join: async ({ request, locals }) => {
    const form = await request.formData();
    const slug = (form.get('slug') ?? '').toString().trim().toLowerCase();
    let joinedSlug: string;
    try {
      const room = await roomsApi.join(serverFetch(locals.sessionCookie), slug);
      joinedSlug = room.slug;
    } catch (e) {
      if (e instanceof ApiCallError) return fail(e.status, { joinError: e.message, joinSlug: slug });
      throw e;
    }
    throw redirect(303, `/rooms/${joinedSlug}`);
  }
};

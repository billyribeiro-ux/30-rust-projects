import { messagesApi, roomsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  const f = serverFetch(locals.sessionCookie);
  try {
    const [room, messages] = await Promise.all([
      roomsApi.get(f, params.slug),
      messagesApi.list(f, params.slug, { limit: 50 })
    ]);
    // Server returns newest first; reverse for chronological display.
    return { room, initialMessages: messages.reverse() };
  } catch (e: unknown) {
    if (e && typeof e === 'object' && 'status' in e && e.status === 404) {
      throw error(404, 'Room not found or you are not a member.');
    }
    throw e;
  }
};

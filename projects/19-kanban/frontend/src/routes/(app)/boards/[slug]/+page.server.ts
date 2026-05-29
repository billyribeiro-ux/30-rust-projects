import { error, fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { ApiCallError, boardsApi } from '$lib/api';

const SESSION_COOKIE = 'app_session';

function withCookie(cookie: string | null): typeof fetch {
  return (input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        ...(cookie ? { cookie: `${SESSION_COOKIE}=${cookie}` } : {})
      }
    });
}

export const load: PageServerLoad = async ({ params, locals }) => {
  const f = withCookie(locals.sessionCookie);
  try {
    const board = await boardsApi.get(f, params.slug);
    return { board };
  } catch (err) {
    if (err instanceof ApiCallError) {
      if (err.status === 404) error(404, 'Board not found.');
      if (err.status === 403) error(403, 'You do not have access to this board.');
    }
    throw err;
  }
};

export const actions: Actions = {
  createList: async ({ request, params, locals }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    if (!name) return fail(422, { error: 'Name is required.' });
    const f = withCookie(locals.sessionCookie);
    try {
      const board = await boardsApi.get(f, params.slug);
      const maxPos = board.lists.reduce((m, l) => Math.max(m, l.position), 0);
      await boardsApi.createList(f, board.id, name, maxPos + 1024);
      return { created: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { error: err.message, fields: err.fields });
      }
      throw err;
    }
  }
};

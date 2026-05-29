import { fail, type Cookies } from '@sveltejs/kit';
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

function setBackendCookie(cookies: Cookies, raw: string | null) {
  if (!raw) return;
  cookies.set(SESSION_COOKIE, raw, {
    path: '/',
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    maxAge: 60 * 60 * 24 * 30
  });
}

export const load: PageServerLoad = async ({ locals }) => {
  const f = withCookie(locals.sessionCookie);
  try {
    const boards = await boardsApi.list(f);
    return { boards };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 401) {
      return { boards: [] };
    }
    throw err;
  }
};

export const actions: Actions = {
  create: async ({ request, locals, cookies }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const slug = String(data.get('slug') ?? '').trim().toLowerCase();
    if (!name || !slug) {
      return fail(422, { error: 'Name and slug are required.', name, slug });
    }
    const f = withCookie(locals.sessionCookie);
    try {
      const board = await boardsApi.create(f, name, slug);
      setBackendCookie(cookies, locals.sessionCookie);
      return { created: board };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, {
          error: err.message,
          fields: err.fields,
          name,
          slug
        });
      }
      return fail(500, { error: 'Unexpected error.', name, slug });
    }
  }
};

import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { postsApi, ApiCallError } from '$lib/api';

const SESSION_COOKIE = 'app_session';

function withCookies(cookie: string | null): typeof fetch {
  return (input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        ...(cookie ? { cookie: `sub_session=${cookie}` } : {})
      }
    });
}

export const load: PageServerLoad = async ({ params, locals, cookies }) => {
  const sub = cookies.get('sub_session') ?? null;
  void SESSION_COOKIE;
  const f = withCookies(sub);
  try {
    const post = await postsApi.get(f, params.slug);
    return { post };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Post not found.');
    if (err instanceof ApiCallError) error(err.status, err.message);
    throw err;
  }
  void locals;
};

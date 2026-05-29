import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { ApiCallError, placesApi } from '$lib/api';

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

export const load: PageServerLoad = async ({ params, locals, url }) => {
  const lat = url.searchParams.get('lat');
  const lng = url.searchParams.get('lng');
  try {
    const place = await placesApi.get(withCookie(locals.sessionCookie), params.id, {
      lat: lat ? Number(lat) : undefined,
      lng: lng ? Number(lng) : undefined
    });
    return { place };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Place not found.');
    throw err;
  }
};

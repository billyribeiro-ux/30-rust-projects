/**
 * Public search page. SSR'd for SEO (Restaurant JSON-LD is emitted on
 * the rendered HTML), so we run the same `placesApi.list` server-side
 * with `event.fetch` so cookies forward if the visitor is signed in.
 */

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

export const load: PageServerLoad = async ({ locals, url }) => {
  const lat = url.searchParams.get('lat');
  const lng = url.searchParams.get('lng');
  const q = url.searchParams.get('q') ?? '';
  const cuisine = url.searchParams.get('cuisine') ?? '';
  const radius = url.searchParams.get('radius_m');

  const f = withCookie(locals.sessionCookie);
  try {
    const places = await placesApi.list(f, {
      lat: lat ? Number(lat) : undefined,
      lng: lng ? Number(lng) : undefined,
      radius_m: radius ? Number(radius) : undefined,
      q: q || undefined,
      cuisine: cuisine || undefined
    });
    return { places, lat, lng, q, cuisine, radius };
  } catch (err) {
    if (err instanceof ApiCallError) {
      return { places: [], lat, lng, q, cuisine, radius, error: err.message };
    }
    return { places: [], lat, lng, q, cuisine, radius, error: 'Search failed.' };
  }
};

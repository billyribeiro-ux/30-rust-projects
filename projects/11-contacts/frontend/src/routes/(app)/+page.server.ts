import { dashboardApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  const data = await dashboardApi.get(serverFetch(locals.sessionCookie));
  return { dashboard: data };
};

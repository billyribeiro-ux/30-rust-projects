import { pollsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  const polls = await pollsApi.list(serverFetch(locals.sessionCookie));
  return { polls };
};

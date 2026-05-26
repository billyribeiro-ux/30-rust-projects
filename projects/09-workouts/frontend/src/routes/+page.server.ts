import { workoutsApi, statsApi } from '$lib/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const [workouts, stats] = await Promise.all([
    workoutsApi.list(fetch),
    statsApi.read(fetch)
  ]);
  return { workouts, stats };
};

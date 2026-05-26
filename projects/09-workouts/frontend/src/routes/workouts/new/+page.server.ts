import { redirect, fail } from '@sveltejs/kit';
import { workoutsApi, exercisesApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const exercises = await exercisesApi.list(fetch);
  return { exercises };
};

type IncomingSet = {
  exercise_id: string;
  weight_minor: number;
  reps: number;
  rir: number;
};

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    let setsRaw: unknown;
    try {
      setsRaw = JSON.parse(String(data.get('sets') ?? '[]'));
    } catch {
      return fail(422, { error: 'Invalid sets payload.' });
    }
    if (!Array.isArray(setsRaw) || setsRaw.length === 0) {
      return fail(422, { error: 'Add at least one set.' });
    }
    const sets: IncomingSet[] = setsRaw.map((raw: unknown) => {
      const r = raw as Record<string, unknown>;
      return {
        exercise_id: String(r.exercise_id ?? ''),
        weight_minor: Number(r.weight_minor ?? 0),
        reps: Number(r.reps ?? 0),
        rir: Number(r.rir ?? 0)
      };
    });

    try {
      const workout = await workoutsApi.create(fetch, { name, sets });
      redirect(303, `/workouts/${workout.id}`);
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { error: err.message });
      }
      throw err;
    }
  }
};

import { fail } from '@sveltejs/kit';
import { exercisesApi, ApiCallError } from '$lib/api';
import type { MuscleGroup } from '$lib/types';
import { MUSCLE_GROUPS } from '$lib/types';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const exercises = await exercisesApi.list(fetch);
  return { exercises };
};

function isMuscleGroup(v: string): v is MuscleGroup {
  return (MUSCLE_GROUPS as readonly string[]).includes(v);
}

export const actions: Actions = {
  add: async ({ request, fetch }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const muscleRaw = String(data.get('muscle_group') ?? 'other').trim();
    const muscle_group: MuscleGroup = isMuscleGroup(muscleRaw) ? muscleRaw : 'other';
    if (!name) return fail(422, { kind: 'add', error: 'Name is required.', name });
    try {
      await exercisesApi.create(fetch, { name, muscle_group });
      return { success: true, kind: 'add' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'add', error: err.message, name });
      }
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'remove', error: 'Missing id.' });
    try {
      await exercisesApi.remove(fetch, id);
      return { success: true, kind: 'remove' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'remove', error: err.message });
      }
      throw err;
    }
  }
};

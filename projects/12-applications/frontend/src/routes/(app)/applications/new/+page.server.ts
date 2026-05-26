import { fail, redirect } from '@sveltejs/kit';
import { applicationsApi, ApiCallError } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { AppStatus } from '$lib/types';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, locals }) => {
    const data = await request.formData();
    const company = String(data.get('company') ?? '').trim();
    const role = String(data.get('role') ?? '').trim();
    const location = String(data.get('location') ?? '').trim();
    const status = String(data.get('status') ?? 'applied') as AppStatus;
    const job_url = String(data.get('job_url') ?? '').trim();
    const notes = String(data.get('notes') ?? '').trim();

    if (!company) return fail(422, { company, role, location, error: 'Company is required.' });
    if (!role) return fail(422, { company, role, location, error: 'Role is required.' });

    try {
      const created = await applicationsApi.create(serverFetch(locals.sessionCookie), {
        company,
        role,
        location,
        status,
        job_url,
        notes
      });
      redirect(303, `/applications/${created.id}`);
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { company, role, location, error: err.message });
      }
      throw err;
    }
  }
};

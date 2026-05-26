import type { Actions } from './$types';

const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3011';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    // The backend ALWAYS returns 204 here whether the user exists or not,
    // so we don't need any error handling — we just show the same success
    // message in both cases. (This is the no-enumeration discipline.)
    try {
      await fetch(`${BACKEND_URL}/api/auth/forgot`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email })
      });
    } catch {
      /* network failure also looks like success to the user */
    }
    return { sent: true };
  }
};

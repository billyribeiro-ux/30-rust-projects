import { ApiCallError, calendarsApi, eventsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

/** Return the first millisecond of the month containing `d`. */
function monthStart(d: Date): Date {
  return new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), 1));
}
function monthEnd(d: Date): Date {
  return new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + 1, 1));
}

export const load: PageServerLoad = async ({ locals, url }) => {
  const f = serverFetch(locals.sessionCookie);
  const monthParam = url.searchParams.get('m'); // YYYY-MM
  const today = new Date();
  const anchor = monthParam
    ? new Date(`${monthParam}-01T00:00:00Z`)
    : new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), 1));
  const from = monthStart(anchor);
  const to = monthEnd(anchor);

  const [calendars, occurrences] = await Promise.all([
    calendarsApi.list(f),
    eventsApi.range(f, from.toISOString(), to.toISOString())
  ]);

  return {
    calendars,
    occurrences,
    month: from.toISOString().slice(0, 7) // YYYY-MM
  };
};

export const actions: Actions = {
  newCalendar: async ({ request, locals }) => {
    const form = await request.formData();
    const name = (form.get('name') ?? '').toString();
    const color = (form.get('color') ?? '#3b82f6').toString();
    const tz = (form.get('tz') ?? 'UTC').toString();
    try {
      await calendarsApi.create(serverFetch(locals.sessionCookie), name, color, tz);
    } catch (e) {
      if (e instanceof ApiCallError) return fail(e.status, { error: e.message, name });
      throw e;
    }
    return { ok: true };
  },
  newEvent: async ({ request, locals }) => {
    const form = await request.formData();
    const calendar_id = (form.get('calendar_id') ?? '').toString();
    const title = (form.get('title') ?? '').toString();
    const start = (form.get('start') ?? '').toString();
    const end = (form.get('end') ?? '').toString();
    const tz = (form.get('tz') ?? 'UTC').toString();
    const rrule = ((form.get('rrule') ?? '').toString() || null) as string | null;

    // Both inputs are datetime-local; the browser submits a "YYYY-MM-DDTHH:MM"
    // string interpreted in the user's local timezone. We parse it as a Date
    // (interpreted as local) and serialise UTC for the wire.
    const startISO = new Date(start).toISOString();
    const endISO = new Date(end).toISOString();

    try {
      await eventsApi.create(serverFetch(locals.sessionCookie), {
        calendar_id,
        title,
        start_at: startISO,
        end_at: endISO,
        tz,
        rrule
      });
    } catch (e) {
      if (e instanceof ApiCallError)
        return fail(e.status, { eventError: e.message, title, start, end });
      throw e;
    }
    return { ok: true };
  }
};

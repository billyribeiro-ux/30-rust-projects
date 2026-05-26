import { fail } from '@sveltejs/kit';
import {
  membersApi,
  expensesApi,
  balancesApi,
  ApiCallError,
  type CreateExpenseInput
} from '$lib/api';
import type { ShareInput, SplitKind } from '$lib/types';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const [members, expenses, balances] = await Promise.all([
    membersApi.list(fetch),
    expensesApi.list(fetch),
    balancesApi.get(fetch)
  ]);
  return { members, expenses, balances };
};

export const actions: Actions = {
  addMember: async ({ request, fetch }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const color = String(data.get('color') ?? '').toUpperCase();
    if (!name) return fail(422, { kind: 'addMember', name, color, error: 'Name is required.' });

    try {
      await membersApi.create(fetch, { name, color });
      return { success: true, kind: 'addMember' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, {
          kind: 'addMember',
          name,
          color,
          error: err.message,
          fields: err.fields
        });
      }
      throw err;
    }
  },

  removeMember: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'removeMember', error: 'Missing id.' });
    try {
      await membersApi.remove(fetch, id);
      return { success: true, kind: 'removeMember' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'removeMember', error: err.message, fields: err.fields });
      }
      throw err;
    }
  },

  addExpense: async ({ request, fetch }) => {
    const data = await request.formData();
    const payload = data.get('payload');
    if (typeof payload !== 'string') {
      return fail(422, { kind: 'addExpense', error: 'Missing payload.' });
    }
    let parsed: CreateExpenseInput;
    try {
      parsed = JSON.parse(payload) as CreateExpenseInput;
    } catch {
      return fail(422, { kind: 'addExpense', error: 'Bad payload.' });
    }

    if (!parsed.payer_id) {
      return fail(422, { kind: 'addExpense', error: 'Choose a payer.' });
    }
    if (!Number.isInteger(parsed.amount_cents) || parsed.amount_cents <= 0) {
      return fail(422, { kind: 'addExpense', error: 'Enter an amount greater than zero.' });
    }
    const allowedKinds: SplitKind[] = ['equal', 'exact', 'percent'];
    if (!allowedKinds.includes(parsed.split_kind)) {
      return fail(422, { kind: 'addExpense', error: 'Invalid split kind.' });
    }
    if (!Array.isArray(parsed.shares) || parsed.shares.length === 0) {
      return fail(422, { kind: 'addExpense', error: 'Pick at least one member to split with.' });
    }
    const cleanShares: ShareInput[] = parsed.shares.map((s) => ({
      member_id: String(s.member_id),
      value: Number.isFinite(s.value) ? Math.trunc(s.value) : 0
    }));

    const input: CreateExpenseInput = {
      payer_id: parsed.payer_id,
      amount_cents: parsed.amount_cents,
      description: String(parsed.description ?? '').slice(0, 200),
      split_kind: parsed.split_kind,
      shares: cleanShares
    };

    try {
      await expensesApi.create(fetch, input);
      return { success: true, kind: 'addExpense' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, {
          kind: 'addExpense',
          error: err.message,
          fields: err.fields
        });
      }
      throw err;
    }
  },

  removeExpense: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'removeExpense', error: 'Missing id.' });
    try {
      await expensesApi.remove(fetch, id);
      return { success: true, kind: 'removeExpense' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'removeExpense', error: err.message });
      }
      throw err;
    }
  }
};

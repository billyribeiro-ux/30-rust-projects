import { fail } from '@sveltejs/kit';
import {
  accountsApi,
  transactionsApi,
  importsApi,
  ApiCallError,
  type CreateTransactionInput
} from '$lib/api';
import type { AccountKind } from '$lib/types';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const [accounts, transactions] = await Promise.all([
    accountsApi.list(fetch),
    transactionsApi.list(fetch)
  ]);
  return { accounts, transactions };
};

export const actions: Actions = {
  addAccount: async ({ request, fetch }) => {
    const data = await request.formData();
    const name = String(data.get('name') ?? '').trim();
    const kind = String(data.get('kind') ?? '') as AccountKind;
    const color = String(data.get('color') ?? '').toUpperCase();
    if (!name) return fail(422, { kind: 'addAccount', error: 'Name is required.' });
    try {
      await accountsApi.create(fetch, { name, kind, color });
      return { success: true, kind: 'addAccount' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'addAccount', error: err.message });
      }
      throw err;
    }
  },

  removeAccount: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'removeAccount', error: 'Missing id.' });
    try {
      await accountsApi.remove(fetch, id);
      return { success: true, kind: 'removeAccount' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'removeAccount', error: err.message });
      }
      throw err;
    }
  },

  addTransaction: async ({ request, fetch }) => {
    const data = await request.formData();
    const payload = data.get('payload');
    if (typeof payload !== 'string') {
      return fail(422, { kind: 'addTransaction', error: 'Missing payload.' });
    }
    let parsed: CreateTransactionInput;
    try {
      parsed = JSON.parse(payload) as CreateTransactionInput;
    } catch {
      return fail(422, { kind: 'addTransaction', error: 'Bad payload.' });
    }
    try {
      await transactionsApi.create(fetch, parsed);
      return { success: true, kind: 'addTransaction' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'addTransaction', error: err.message });
      }
      throw err;
    }
  },

  removeTransaction: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'removeTransaction', error: 'Missing id.' });
    try {
      await transactionsApi.remove(fetch, id);
      return { success: true, kind: 'removeTransaction' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'removeTransaction', error: err.message });
      }
      throw err;
    }
  },

  importCsv: async ({ request, fetch }) => {
    const data = await request.formData();
    const csv = String(data.get('csv') ?? '');
    const account = String(data.get('asset_account_name') ?? '');
    if (!csv) return fail(422, { kind: 'importCsv', error: 'Paste a CSV body.' });
    if (!account) return fail(422, { kind: 'importCsv', error: 'Pick a target account.' });
    try {
      const result = await importsApi.csv(fetch, csv, account);
      return { success: true, kind: 'importCsv', result };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'importCsv', error: err.message });
      }
      throw err;
    }
  }
};

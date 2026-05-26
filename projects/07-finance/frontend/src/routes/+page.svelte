<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, Trash, CurrencyDollar, ChartBar } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AccountRow from '$lib/components/AccountRow.svelte';
  import BarChart from '$lib/components/BarChart.svelte';
  import { formatMoney, parseMoney, formatCents } from '$lib/money';
  import { ALLOWED_COLORS, KIND_LABEL, type AccountKind } from '$lib/types';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  const accounts = $derived(data.accounts);
  const transactions = $derived(data.transactions);

  // --- Add account form
  let acctName = $state('');
  let acctKind = $state<AccountKind>('expense');
  let acctColor = $state<string>(ALLOWED_COLORS[0]);

  // --- Add transaction form (two-posting "simple" entry)
  let txDescription = $state('');
  let fromId = $state('');
  let toId = $state('');
  let amount = $state<number | null>(null);
  let amountText = $state('');

  function onAmountInput(e: Event) {
    amountText = (e.target as HTMLInputElement).value;
    amount = parseMoney(amountText);
  }

  // The payload that the form submits — built as a derived JSON string.
  // For a "simple" two-posting transaction: amount flows FROM one account
  // TO another (e.g., expense -> bank withdrawal). The "from" account gets
  // a negative posting; the "to" account gets a positive one. That's the
  // double-entry convention we use throughout.
  const payloadJson = $derived(() => {
    if (!amount || !fromId || !toId || fromId === toId) {
      return JSON.stringify({ description: txDescription, occurred_at: '', postings: [] });
    }
    return JSON.stringify({
      description: txDescription,
      occurred_at: new Date().toISOString(),
      postings: [
        { account_id: fromId, amount_minor: -amount },
        { account_id: toId, amount_minor: amount }
      ]
    });
  });

  const txReady = $derived(!!amount && amount > 0 && !!fromId && !!toId && fromId !== toId);

  // --- Bar chart data — balance by expense account (most spent)
  const chartData = $derived(
    accounts
      .filter((a) => a.kind === 'expense')
      .map((a) => ({
        label: a.name,
        value: Math.abs(a.balance_minor),
        color: a.color
      }))
      .sort((a, b) => b.value - a.value)
      .slice(0, 10)
  );

  function accountById(id: string) {
    return accounts.find((a) => a.id === id);
  }

  function deltaForAccount(tx: (typeof transactions)[number], accountId: string): number {
    return tx.postings
      .filter((p) => p.account_id === accountId)
      .reduce((acc, p) => acc + p.amount_minor, 0);
  }

  // Hidden delete forms
  let deleteAccountForm: HTMLFormElement | undefined = $state();
  let deleteAccountId = $state('');
  let deleteTxForm: HTMLFormElement | undefined = $state();
  let deleteTxId = $state('');

  function handleRemoveAccount(id: string) {
    deleteAccountId = id;
    queueMicrotask(() => deleteAccountForm?.requestSubmit());
  }
  function handleRemoveTx(id: string) {
    deleteTxId = id;
    queueMicrotask(() => deleteTxForm?.requestSubmit());
  }

  const addTxError = $derived(
    form && (form as { kind?: string }).kind === 'addTransaction' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
  const addAcctError = $derived(
    form && (form as { kind?: string }).kind === 'addAccount' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );

  // --- CSV import
  let importCsvText = $state('');
  let importAccountName = $state('');
  const importResult = $derived(
    form && (form as { kind?: string }).kind === 'importCsv' && 'result' in form
      ? (form as { result?: { imported: number; skipped: number; errors: string[] } }).result
      : undefined
  );
  const importError = $derived(
    form && (form as { kind?: string }).kind === 'importCsv' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={CurrencyDollar} size={28} weight="duotone" />
      <h1>Ledger</h1>
    </div>
    <p class="lead">Double-entry personal finance. Every transaction balances.</p>
  </header>

  <section class="panel" aria-label="Accounts">
    <h2>Accounts</h2>
    {#if accounts.length === 0}
      <p class="empty">Add your first account below to get started.</p>
    {:else}
      <ul class="accounts">
        {#each accounts as a (a.id)}
          <AccountRow account={a} onDelete={handleRemoveAccount} />
        {/each}
      </ul>
    {/if}

    <form
      method="POST"
      action="?/addAccount"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') acctName = '';
        await update();
      }}
      class="form-grid"
    >
      <input
        type="text"
        name="name"
        bind:value={acctName}
        placeholder="Account name (e.g., Checking, Groceries)"
        maxlength="60"
        required
        aria-label="New account name"
      />
      <select bind:value={acctKind} name="kind" aria-label="Account kind">
        {#each Object.entries(KIND_LABEL) as [k, label] (k)}
          <option value={k}>{label}</option>
        {/each}
      </select>
      <fieldset class="palette" aria-label="Color">
        <legend class="sr-only">Color</legend>
        {#each ALLOWED_COLORS as c (c)}
          <label class="swatch" style="--swatch: {c}">
            <input type="radio" name="color" value={c} bind:group={acctColor} />
            <span aria-hidden="true"></span>
            <span class="sr-only">{c}</span>
          </label>
        {/each}
      </fieldset>
      <button type="submit" class="primary">
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Add account</span>
      </button>
      {#if addAcctError}
        <p class="error" role="alert">{addAcctError}</p>
      {/if}
    </form>
  </section>

  <section class="panel" aria-label="New transaction">
    <h2>New transaction</h2>
    {#if accounts.length < 2}
      <p class="empty">Add at least two accounts to record a transaction.</p>
    {:else}
      <form
        method="POST"
        action="?/addTransaction"
        use:enhance={() => async ({ result, update }) => {
          if (result.type === 'success') {
            txDescription = '';
            amount = null;
            amountText = '';
          }
          await update();
        }}
      >
        <div class="row two">
          <label class="field">
            <span>From (credited / negative)</span>
            <select bind:value={fromId} required>
              <option value="" disabled>Choose account…</option>
              {#each accounts as a (a.id)}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            <span>To (debited / positive)</span>
            <select bind:value={toId} required>
              <option value="" disabled>Choose account…</option>
              {#each accounts as a (a.id)}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </label>
        </div>

        <label class="field">
          <span>Amount</span>
          <div class="input-wrap">
            <span class="prefix" aria-hidden="true">$</span>
            <input
              type="text"
              inputmode="decimal"
              value={amountText}
              oninput={onAmountInput}
              placeholder="0.00"
              required
              aria-label="Amount"
            />
          </div>
        </label>

        <label class="field">
          <span>Description (optional)</span>
          <input
            type="text"
            bind:value={txDescription}
            placeholder="e.g., grocery store, salary, dinner out"
            maxlength="200"
          />
        </label>

        {#if fromId && toId && fromId === toId}
          <p class="error" role="alert">From and To must be different accounts.</p>
        {/if}

        {#if addTxError}
          <p class="error" role="alert">{addTxError}</p>
        {/if}

        <input type="hidden" name="payload" value={payloadJson()} />
        <button type="submit" class="primary big" disabled={!txReady}>
          <Icon icon={Plus} size={18} weight="bold" />
          <span>Record transaction</span>
        </button>
      </form>
    {/if}
  </section>

  <section class="panel" aria-label="Top expenses chart">
    <h2>
      <Icon icon={ChartBar} size={18} weight="bold" />
      <span>Top expenses</span>
    </h2>
    <BarChart data={chartData} ariaLabel="Top expense balances" />
  </section>

  <section class="panel" aria-label="Transaction history">
    <h2>History</h2>
    {#if transactions.length === 0}
      <p class="empty">No transactions yet.</p>
    {:else}
      <ul class="history">
        {#each transactions as tx (tx.id)}
          <li>
            <div class="head">
              <p class="desc">{tx.description || '(no description)'}</p>
              <p class="when">{new Date(tx.occurred_at).toLocaleDateString()}</p>
            </div>
            <ul class="postings">
              {#each tx.postings as p (p.id)}
                {@const acct = accountById(p.account_id)}
                <li>
                  <span class="dot" style="background: {acct?.color}" aria-hidden="true"></span>
                  <span class="acct">{acct?.name ?? '?'}</span>
                  <span class="amt" class:debit={p.amount_minor < 0}>{formatMoney(p.amount_minor)}</span>
                </li>
              {/each}
            </ul>
            <button
              type="button"
              class="del"
              aria-label="Delete transaction"
              onclick={() => handleRemoveTx(tx.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel" aria-label="CSV import">
    <h2>Import CSV</h2>
    <p class="hint">
      Columns: <code>date,description,amount,counterparty_account</code> (account names must
      match existing accounts). Sample line: <code>2026-05-24,Groceries,-42.50,Groceries</code>.
    </p>
    <form
      method="POST"
      action="?/importCsv"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') {
          importCsvText = '';
        }
        await update();
      }}
    >
      <label class="field">
        <span>Target account (asset side)</span>
        <select bind:value={importAccountName} name="asset_account_name" required>
          <option value="" disabled>Choose account…</option>
          {#each accounts.filter((a) => a.kind === 'asset') as a (a.id)}
            <option value={a.name}>{a.name}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>CSV body</span>
        <textarea
          name="csv"
          bind:value={importCsvText}
          rows="6"
          placeholder={'date,description,amount,counterparty_account\n2026-05-24,Groceries,-42.50,Groceries'}
        ></textarea>
      </label>
      {#if importError}
        <p class="error" role="alert">{importError}</p>
      {/if}
      {#if importResult}
        <p class="ok" role="status">
          Imported {importResult.imported}, skipped {importResult.skipped}.
        </p>
        {#if importResult.errors.length > 0}
          <ul class="import-errors">
            {#each importResult.errors as msg, i (i)}
              <li>{msg}</li>
            {/each}
          </ul>
        {/if}
      {/if}
      <button type="submit" class="primary">Import</button>
    </form>
  </section>

  <form
    bind:this={deleteAccountForm}
    method="POST"
    action="?/removeAccount"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteAccountId} />
  </form>
  <form
    bind:this={deleteTxForm}
    method="POST"
    action="?/removeTransaction"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteTxId} />
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }

  header {
    display: grid;
    gap: var(--space-2);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
  }
  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
  }
  .lead {
    color: var(--color-fg-muted);
  }

  .panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-4);
  }

  h2 {
    font-size: var(--text-lg);
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .accounts {
    display: grid;
    gap: var(--space-2);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .form-grid {
    display: grid;
    gap: var(--space-3);
  }

  .form-grid input[type='text'],
  .form-grid select {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .palette {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    border: 0;
    padding: 0;
  }
  .swatch {
    position: relative;
    width: 28px;
    height: 28px;
    cursor: pointer;
  }
  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
  }
  .swatch span[aria-hidden] {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 999px;
    background: var(--swatch);
    border: 2px solid transparent;
  }
  .swatch input:checked + span[aria-hidden] {
    border-color: var(--color-fg);
    transform: scale(1.1);
  }

  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    justify-self: start;
  }
  .primary.big {
    width: 100%;
    justify-content: center;
  }
  .primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .row.two {
    display: grid;
    gap: var(--space-3);
  }

  .field {
    display: grid;
    gap: var(--space-1);
  }
  .field > span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .field input,
  .field select,
  .field textarea {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-family: var(--font-sans);
  }
  .field textarea {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    resize: vertical;
  }
  .input-wrap {
    position: relative;
  }
  .input-wrap .prefix {
    position: absolute;
    top: 50%;
    left: var(--space-3);
    transform: translateY(-50%);
    color: var(--color-fg-muted);
  }
  .input-wrap input {
    padding-left: calc(var(--space-3) + 12px);
    width: 100%;
    font-variant-numeric: tabular-nums;
  }

  .history {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }
  .history li {
    position: relative;
    padding: var(--space-3) var(--space-4);
    padding-right: var(--space-10);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    align-items: baseline;
  }
  .head .desc {
    font-weight: 600;
  }
  .head .when {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
  }

  .postings {
    list-style: none;
    padding: 0;
    margin: var(--space-2) 0 0 0;
    display: grid;
    gap: var(--space-1);
  }
  .postings li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: var(--space-2);
    align-items: center;
    padding: 0;
    background: transparent;
  }
  .postings .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }
  .acct {
    font-size: var(--text-sm);
  }
  .amt {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: hsl(142 71% 28%);
  }
  .amt.debit {
    color: hsl(0 72% 42%);
  }

  .del {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
  }
  .del:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }

  .empty {
    color: var(--color-fg-muted);
    padding: var(--space-4);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    text-align: center;
  }

  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .ok {
    padding: var(--space-2) var(--space-3);
    background: hsl(142 71% 35% / 0.12);
    color: hsl(142 71% 28%);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }

  .import-errors {
    list-style: disc inside;
    padding: 0;
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
  }

  .hint {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .hint code {
    font-family: var(--font-mono);
    background: var(--color-bg-sunken);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    font-size: 0.9em;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }
    .row.two {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>

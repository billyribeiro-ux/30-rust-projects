<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, Trash, ArrowRight, CurrencyDollar } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import MoneyInput from '$lib/components/MoneyInput.svelte';
  import MemberPill from '$lib/components/MemberPill.svelte';
  import { ALLOWED_COLORS, type SplitKind } from '$lib/types';
  import { formatMoney, formatCents, parseMoney } from '$lib/money';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  const members = $derived(data.members);
  const expenses = $derived(data.expenses);
  const balances = $derived(data.balances.balances);
  const settlements = $derived(data.balances.settlements);

  // --- Add member form ---
  let memberName = $state('');
  let memberColor = $state<string>(ALLOWED_COLORS[0]);

  // --- Add expense form ---
  let payerId = $state('');
  let amount = $state<number | null>(null);
  let description = $state('');
  let splitKind = $state<SplitKind>('equal');
  // selected member ids (Set keyed by id) — for `equal` split
  let selectedIds = $state<string[]>([]);
  // For `exact` split: map of member_id → cents
  let exactCents = $state<Record<string, number>>({});
  // For `percent` split: map of member_id → basis points (0..10000)
  let percentBp = $state<Record<string, number>>({});

  // Recompute selectedIds whenever members change so the default is "all"
  $effect(() => {
    if (members.length > 0 && selectedIds.length === 0) {
      selectedIds = members.map((m) => m.id);
    }
  });

  // Live preview of how the split would land
  const preview = $derived(computePreview());

  function computePreview(): { ok: boolean; rows: { id: string; cents: number }[]; sum: number } {
    if (amount === null || amount <= 0) return { ok: false, rows: [], sum: 0 };
    if (selectedIds.length === 0) return { ok: false, rows: [], sum: 0 };

    if (splitKind === 'equal') {
      const n = selectedIds.length;
      const base = Math.floor(amount / n);
      const remainder = amount - base * n;
      const rows = selectedIds.map((id, i) => ({
        id,
        cents: base + (i < remainder ? 1 : 0)
      }));
      const sum = rows.reduce((acc, r) => acc + r.cents, 0);
      return { ok: sum === amount, rows, sum };
    }

    if (splitKind === 'exact') {
      const rows = selectedIds.map((id) => ({ id, cents: exactCents[id] ?? 0 }));
      const sum = rows.reduce((acc, r) => acc + r.cents, 0);
      return { ok: sum === amount, rows, sum };
    }

    // percent — distribute leftover cent to largest fractional remainders
    const total = amount;
    const items = selectedIds.map((id, i) => {
      const bp = percentBp[id] ?? 0;
      const exact = total * bp;
      const cents = Math.floor(exact / 10_000);
      const rem = exact - cents * 10_000;
      return { i, id, bp, cents, rem };
    });
    const totalBp = items.reduce((acc, it) => acc + it.bp, 0);
    if (totalBp !== 10_000) {
      return { ok: false, rows: items.map((it) => ({ id: it.id, cents: it.cents })), sum: 0 };
    }
    const assigned = items.reduce((acc, it) => acc + it.cents, 0);
    let leftover = amount - assigned;
    const order = [...items].sort((a, b) => b.rem - a.rem || a.i - b.i);
    let k = 0;
    while (leftover > 0) {
      const pos = order[k % order.length]!;
      pos.cents += 1;
      leftover -= 1;
      k += 1;
    }
    const rows = items.map((it) => ({ id: it.id, cents: it.cents }));
    const sum = rows.reduce((acc, r) => acc + r.cents, 0);
    return { ok: sum === amount, rows, sum };
  }

  // The hidden field that holds the serialized payload (so we use a single
  // form action and submit the whole thing as JSON).
  const payloadJson = $derived(() => {
    const shares = selectedIds.map((id) => ({
      member_id: id,
      value:
        splitKind === 'exact'
          ? exactCents[id] ?? 0
          : splitKind === 'percent'
            ? percentBp[id] ?? 0
            : 0
    }));
    return JSON.stringify({
      payer_id: payerId,
      amount_cents: amount ?? 0,
      description,
      split_kind: splitKind,
      shares
    });
  });

  function memberById(id: string) {
    return members.find((m) => m.id === id);
  }

  function toggleMember(id: string) {
    if (selectedIds.includes(id)) selectedIds = selectedIds.filter((s) => s !== id);
    else selectedIds = [...selectedIds, id];
  }

  function setPercent(id: string, pctText: string) {
    const pct = Number(pctText);
    if (!Number.isFinite(pct)) return;
    percentBp = { ...percentBp, [id]: Math.round(pct * 100) };
  }

  function setExact(id: string, cents: number | null) {
    exactCents = { ...exactCents, [id]: cents ?? 0 };
  }

  // Hidden delete forms (one each)
  let deleteMemberForm: HTMLFormElement | undefined = $state();
  let deleteMemberId = $state('');
  let deleteExpenseForm: HTMLFormElement | undefined = $state();
  let deleteExpenseId = $state('');

  function handleRemoveMember(id: string) {
    deleteMemberId = id;
    queueMicrotask(() => deleteMemberForm?.requestSubmit());
  }

  function handleRemoveExpense(id: string) {
    deleteExpenseId = id;
    queueMicrotask(() => deleteExpenseForm?.requestSubmit());
  }

  const addExpenseError = $derived(
    form && (form as { kind?: string }).kind === 'addExpense' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
  const addMemberError = $derived(
    form && (form as { kind?: string }).kind === 'addMember' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={CurrencyDollar} size={28} weight="duotone" />
      <h1>Splitter</h1>
    </div>
    <p class="lead">Add household members, log shared expenses, see who owes whom.</p>
  </header>

  <section aria-label="Members" class="panel">
    <h2>Members</h2>
    <ul class="members">
      {#each members as m (m.id)}
        <li>
          <MemberPill member={m} />
          <button
            type="button"
            class="icon-btn"
            aria-label={`Remove ${m.name}`}
            onclick={() => handleRemoveMember(m.id)}
          >
            <Icon icon={Trash} size={14} />
          </button>
        </li>
      {/each}
    </ul>

    <form
      method="POST"
      action="?/addMember"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') {
          memberName = '';
        }
        await update();
      }}
      class="add-member"
    >
      <input
        type="text"
        name="name"
        bind:value={memberName}
        placeholder="Add a member"
        maxlength="60"
        required
        aria-label="New member name"
      />
      <fieldset class="palette" aria-label="Color">
        <legend class="sr-only">Color</legend>
        {#each ALLOWED_COLORS as c (c)}
          <label class="swatch" style="--swatch: {c}">
            <input type="radio" name="color" value={c} bind:group={memberColor} />
            <span aria-hidden="true"></span>
            <span class="sr-only">{c}</span>
          </label>
        {/each}
      </fieldset>
      <button type="submit" class="primary">
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Add</span>
      </button>
      {#if addMemberError}
        <p class="error" role="alert">{addMemberError}</p>
      {/if}
    </form>
  </section>

  <section aria-label="Add expense" class="panel">
    <h2>New expense</h2>

    {#if members.length < 1}
      <p class="empty">Add at least one member above to log an expense.</p>
    {:else}
      <form
        method="POST"
        action="?/addExpense"
        use:enhance={() => async ({ result, update }) => {
          if (result.type === 'success') {
            amount = null;
            description = '';
            exactCents = {};
            percentBp = {};
          }
          await update();
        }}
      >
        <div class="row two">
          <label class="field">
            <span>Who paid</span>
            <select bind:value={payerId} required>
              <option value="" disabled>Choose payer…</option>
              {#each members as m (m.id)}
                <option value={m.id}>{m.name}</option>
              {/each}
            </select>
          </label>

          <MoneyInput bind:cents={amount} label="Amount" name="amount" required />
        </div>

        <label class="field">
          <span>Description (optional)</span>
          <input
            type="text"
            bind:value={description}
            placeholder="e.g., groceries, electric bill"
            maxlength="200"
          />
        </label>

        <fieldset class="kind-group" aria-label="Split kind">
          <legend>Split kind</legend>
          <label><input type="radio" bind:group={splitKind} value="equal" /> <span>Equally</span></label>
          <label><input type="radio" bind:group={splitKind} value="exact" /> <span>Exact amounts</span></label>
          <label><input type="radio" bind:group={splitKind} value="percent" /> <span>Percentages</span></label>
        </fieldset>

        <fieldset class="split-members" aria-label="Who shares this expense">
          <legend>Members in this expense</legend>
          {#each members as m (m.id)}
            {@const selected = selectedIds.includes(m.id)}
            <div class="split-row" class:dim={!selected}>
              <label class="check">
                <input
                  type="checkbox"
                  checked={selected}
                  onchange={() => toggleMember(m.id)}
                />
                <MemberPill member={m} />
              </label>
              {#if selected && splitKind === 'exact'}
                <input
                  type="text"
                  inputmode="decimal"
                  aria-label={`${m.name} exact share`}
                  value={formatCents(exactCents[m.id] ?? 0)}
                  oninput={(e) =>
                    setExact(m.id, parseMoney((e.target as HTMLInputElement).value))}
                  class="inline-amount"
                />
              {:else if selected && splitKind === 'percent'}
                <div class="percent-wrap">
                  <input
                    type="number"
                    min="0"
                    max="100"
                    step="0.01"
                    aria-label={`${m.name} percent`}
                    value={((percentBp[m.id] ?? 0) / 100).toFixed(2)}
                    oninput={(e) => setPercent(m.id, (e.target as HTMLInputElement).value)}
                  />
                  <span>%</span>
                </div>
              {:else if selected}
                <span class="preview-amount">
                  {formatMoney(preview.rows.find((r) => r.id === m.id)?.cents ?? 0)}
                </span>
              {/if}
            </div>
          {/each}
        </fieldset>

        <div class="check-sum" class:ok={preview.ok} class:warn={!preview.ok && amount}>
          {#if !preview.ok && amount}
            <span>
              Shares sum to {formatMoney(preview.sum)} of {formatMoney(amount)}
              ({splitKind === 'percent' ? 'percents must total 100%' : 'amounts must match'})
            </span>
          {:else if preview.ok}
            <span>Shares total {formatMoney(preview.sum)} — looks good.</span>
          {/if}
        </div>

        {#if addExpenseError}
          <p class="error" role="alert">{addExpenseError}</p>
        {/if}

        <input type="hidden" name="payload" value={payloadJson()} />
        <button
          type="submit"
          class="primary big"
          disabled={!preview.ok || !payerId}
        >
          <Icon icon={Plus} size={18} weight="bold" />
          <span>Add expense</span>
        </button>
      </form>
    {/if}
  </section>

  <section aria-label="Balances" class="panel">
    <h2>Balances</h2>
    {#if balances.length === 0}
      <p class="empty">No balances yet.</p>
    {:else}
      <ul class="balances">
        {#each balances as b (b.member_id)}
          <li>
            <span class="balance-name">{b.name}</span>
            <span
              class="balance-net"
              class:credit={b.net_cents > 0}
              class:debit={b.net_cents < 0}
            >
              {b.net_cents > 0 ? '+' : ''}{formatMoney(b.net_cents)}
            </span>
          </li>
        {/each}
      </ul>

      {#if settlements.length > 0}
        <h3>Suggested settlements</h3>
        <ul class="settlements">
          {#each settlements as s (s.from + s.to)}
            {@const fromM = memberById(s.from)}
            {@const toM = memberById(s.to)}
            <li>
              <span>{fromM?.name ?? s.from}</span>
              <Icon icon={ArrowRight} size={14} weight="bold" />
              <span>{toM?.name ?? s.to}</span>
              <span class="amount">{formatMoney(s.cents)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

  <section aria-label="Expense history" class="panel">
    <h2>History</h2>
    {#if expenses.length === 0}
      <p class="empty">No expenses yet.</p>
    {:else}
      <ul class="history">
        {#each expenses as e (e.id)}
          {@const payer = memberById(e.payer_id)}
          <li>
            <div class="head">
              <p class="amount">{formatMoney(e.amount_cents)}</p>
              <p class="desc">{e.description || '(no description)'}</p>
            </div>
            <p class="meta">
              {payer?.name ?? '?'} paid · {e.split_kind} split · {e.shares.length} members
            </p>
            <button
              type="button"
              class="icon-btn"
              aria-label={`Delete expense ${e.description || e.amount_cents}`}
              onclick={() => handleRemoveExpense(e.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <form
    bind:this={deleteMemberForm}
    method="POST"
    action="?/removeMember"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteMemberId} />
  </form>
  <form
    bind:this={deleteExpenseForm}
    method="POST"
    action="?/removeExpense"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteExpenseId} />
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
  }

  h3 {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-top: var(--space-3);
  }

  .members {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .members li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }

  .add-member {
    display: grid;
    gap: var(--space-3);
  }

  .add-member input[type='text'] {
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
    padding: var(--space-3) var(--space-5);
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
  .field select {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .kind-group {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
    border: 0;
    padding: 0;
  }

  .kind-group legend {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
    margin-bottom: var(--space-2);
  }

  .kind-group label {
    display: inline-flex;
    gap: var(--space-2);
    align-items: center;
    cursor: pointer;
  }

  .split-members {
    display: grid;
    gap: var(--space-2);
    border: 1px solid var(--color-border);
    padding: var(--space-3);
    border-radius: var(--radius-md);
  }

  .split-members legend {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
    padding: 0 var(--space-2);
  }

  .split-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: var(--space-3);
  }

  .split-row.dim {
    opacity: 0.5;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    cursor: pointer;
  }

  .preview-amount {
    color: var(--color-fg-muted);
    font-variant-numeric: tabular-nums;
    font-size: var(--text-sm);
  }

  .inline-amount {
    padding: var(--space-1) var(--space-2);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    width: 100px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .percent-wrap {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .percent-wrap input {
    padding: var(--space-1) var(--space-2);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    width: 80px;
    text-align: right;
  }

  .check-sum {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }

  .check-sum.ok {
    background: hsl(142 71% 35% / 0.1);
    color: hsl(142 71% 28%);
  }

  .check-sum.warn {
    background: hsl(38 92% 45% / 0.12);
    color: hsl(38 92% 30%);
  }

  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .balances,
  .settlements,
  .history {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-2);
  }

  .balances li {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
  }

  .balance-net {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .balance-net.credit {
    color: hsl(142 71% 28%);
  }

  .balance-net.debit {
    color: hsl(0 72% 42%);
  }

  .settlements li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
  }

  .settlements .amount {
    margin-left: auto;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .history li {
    position: relative;
    padding: var(--space-3) var(--space-4);
    padding-right: var(--space-10);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
  }

  .history .head {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .history .amount {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .history .desc {
    color: var(--color-fg);
    font-size: var(--text-sm);
  }

  .history .meta {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    margin-top: 2px;
  }

  .history .icon-btn {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
  }

  .empty {
    color: var(--color-fg-muted);
    padding: var(--space-4);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    text-align: center;
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
    .kind-group {
      gap: var(--space-6);
    }
  }
</style>

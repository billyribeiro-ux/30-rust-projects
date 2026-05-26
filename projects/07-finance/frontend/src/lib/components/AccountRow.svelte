<script lang="ts">
  import { Trash } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import type { Account } from '$lib/types';
  import { KIND_LABEL } from '$lib/types';
  import { formatMoney } from '$lib/money';

  type Props = { account: Account; onDelete: (id: string) => void };
  let { account, onDelete }: Props = $props();
</script>

<li class="row" style="--accent: {account.color}">
  <span class="dot" aria-hidden="true"></span>
  <div class="meta">
    <p class="name">{account.name}</p>
    <p class="kind">{KIND_LABEL[account.kind]}</p>
  </div>
  <p class="balance" class:debit={account.balance_minor < 0}>
    {formatMoney(account.balance_minor)}
  </p>
  <button
    type="button"
    class="del"
    aria-label={`Delete account ${account.name}`}
    onclick={() => onDelete(account.id)}
  >
    <Icon icon={Trash} size={14} />
  </button>
</li>

<style>
  .row {
    list-style: none;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: var(--accent);
  }
  .name {
    font-weight: 600;
  }
  .kind {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
  }
  .balance {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: hsl(142 71% 28%);
  }
  .balance.debit {
    color: hsl(0 72% 42%);
  }
  .del {
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
</style>

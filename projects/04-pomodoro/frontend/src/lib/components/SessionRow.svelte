<script lang="ts">
  import { Trash } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import { KIND_COLOR, KIND_LABEL, type Session } from '$lib/types';
  import { formatMinutes, formatTimeOfDay } from '$lib/format';

  type Props = {
    session: Session;
    onDelete: (id: string) => void;
  };

  let { session, onDelete }: Props = $props();
</script>

<li class="row" style="--accent: {KIND_COLOR[session.kind]}">
  <span class="dot" aria-hidden="true"></span>
  <div class="meta">
    <p class="kind">{KIND_LABEL[session.kind]}</p>
    {#if session.label}
      <p class="label" title={session.label}>{session.label}</p>
    {/if}
  </div>
  <p class="duration">{formatMinutes(session.actual_seconds)}</p>
  <p class="time">
    <time datetime={session.started_at}>{formatTimeOfDay(session.started_at)}</time>
  </p>
  <button
    type="button"
    class="delete"
    aria-label={`Delete ${KIND_LABEL[session.kind]} session`}
    onclick={() => onDelete(session.id)}
  >
    <Icon icon={Trash} size={16} />
  </button>
</li>

<style>
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    list-style: none;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: var(--accent);
  }

  .meta {
    min-width: 0;
  }

  .kind {
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .label {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .duration {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--accent);
  }

  .time {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .delete {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
  }

  .delete:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }
</style>

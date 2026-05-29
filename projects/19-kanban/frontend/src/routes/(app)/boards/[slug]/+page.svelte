<script lang="ts">
  import { enhance } from '$app/forms';
  import KanbanBoard from '$lib/components/KanbanBoard.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let creating = $state(false);
</script>

<svelte:head>
  <title>{data.board.name} — Kanban</title>
</svelte:head>

<div class="bar">
  <a href="/boards" class="back">← Boards</a>
  <h1>{data.board.name}</h1>
  <span class="role">role: {data.board.role}</span>
  {#if data.board.role !== 'viewer'}
    <form
      method="POST"
      action="?/createList"
      class="add-list"
      use:enhance={() => {
        creating = true;
        return async ({ update }) => {
          creating = false;
          await update();
        };
      }}
    >
      <input name="name" placeholder="New list name" required maxlength="200" />
      <button type="submit" disabled={creating}>{creating ? '…' : 'Add list'}</button>
    </form>
  {/if}
</div>

{#if form && 'error' in form && form.error}
  <p class="error" role="alert">{form.error as string}</p>
{/if}

<KanbanBoard board={data.board} />

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border-bottom: 1px solid var(--color-border);
    flex-wrap: wrap;
  }
  .back {
    color: var(--color-fg-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  .back:hover {
    color: var(--color-fg);
  }
  h1 {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .role {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    padding: 0 var(--space-2);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-pill);
    text-transform: uppercase;
  }
  .add-list {
    margin-left: auto;
    display: flex;
    gap: var(--space-2);
  }
  .add-list input {
    padding: var(--space-1) var(--space-2);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .add-list button {
    padding: var(--space-1) var(--space-3);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .error {
    margin: var(--space-3) var(--space-4) 0;
    padding: var(--space-2) var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
</style>

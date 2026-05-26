<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import { STATUS_LABEL, type AppStatus } from '$lib/types';

  let { data }: PageProps = $props();
  const apps = $derived(data.apps);

  const STATUSES: AppStatus[] = [
    'wishlist',
    'applied',
    'screening',
    'interview',
    'offer',
    'accepted',
    'rejected',
    'withdrawn'
  ];

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');
  function handleDelete(id: string, label: string) {
    if (!confirm(`Delete "${label}"?`)) return;
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }
</script>

<svelte:head><title>Applications</title></svelte:head>

<main>
  <header>
    <h1>Applications</h1>
    <a class="primary" href="/applications/new">Add application</a>
  </header>

  <nav class="filter" aria-label="Status filter">
    <a href="/applications" class:active={!data.status}>All</a>
    {#each STATUSES as s (s)}
      <a href="/applications?status={s}" class:active={data.status === s}>{STATUS_LABEL[s]}</a>
    {/each}
  </nav>

  {#if apps.length === 0}
    <p class="empty">
      {#if data.status}
        No applications with status "{STATUS_LABEL[data.status]}".
      {:else}
        No applications yet. <a href="/applications/new">Add your first</a>.
      {/if}
    </p>
  {:else}
    <ul class="list">
      {#each apps as a (a.id)}
        <li>
          <div class="meta">
            <a class="title" href="/applications/{a.id}">{a.company} — {a.role}</a>
            {#if a.location}<p class="location">{a.location}</p>{/if}
            <p class="status status-{a.status}">{STATUS_LABEL[a.status]}</p>
          </div>
          <button
            type="button"
            class="del"
            aria-label={`Delete ${a.company}`}
            onclick={() => handleDelete(a.id, `${a.company} — ${a.role}`)}
          >Delete</button>
        </li>
      {/each}
    </ul>
  {/if}

  <form
    bind:this={deleteForm}
    method="POST"
    action="?/remove"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteId} />
  </form>
</main>

<style>
  main { max-width: var(--container-lg); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--space-3); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .filter { display: flex; flex-wrap: wrap; gap: var(--space-2); }
  .filter a {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg-muted);
    background: var(--color-bg-elev);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    text-decoration: none;
  }
  .filter a.active {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }
  .list { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .list li {
    display: flex; justify-content: space-between; align-items: flex-start;
    gap: var(--space-3); padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md);
  }
  .title { font-weight: 600; }
  .location { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .status { font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-fg-muted); margin-top: var(--space-1); }
  .status.status-offer, .status.status-accepted { color: hsl(142 71% 28%); }
  .status.status-rejected, .status.status-withdrawn { color: hsl(0 72% 42%); }
  .del { color: var(--color-fg-muted); font-size: var(--text-xs); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); }
  .del:hover { color: hsl(0 72% 42%); background: var(--color-bg-sunken); }
  .empty { padding: var(--space-12) var(--space-4); text-align: center; color: var(--color-fg-muted); background: var(--color-bg-sunken); border-radius: var(--radius-md); }
</style>

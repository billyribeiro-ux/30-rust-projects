<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head>
  <title>Boards — Kanban</title>
</svelte:head>

<section class="page">
  <header>
    <h1>Boards</h1>
    <p class="lead">Each board has its own lists, cards, and members.</p>
  </header>

  <form
    method="POST"
    action="?/create"
    class="create"
    aria-label="Create board"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => {
        submitting = false;
        await update();
      };
    }}
  >
    <label class="field">
      <span>Name</span>
      <input name="name" required maxlength="200" value={(form?.name as string) ?? ''} />
    </label>
    <label class="field">
      <span>Slug</span>
      <input
        name="slug"
        required
        pattern={'[a-z0-9-]{1,80}'}
        title="lowercase letters, digits, dashes (1-80 chars)"
        value={(form?.slug as string) ?? ''}
      />
    </label>
    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Creating…' : 'Create board'}
    </button>
    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error as string}</p>
    {/if}
  </form>

  {#if data.boards.length === 0}
    <p class="empty">No boards yet. Create your first one above.</p>
  {:else}
    <ul class="boards">
      {#each data.boards as board (board.id)}
        <li>
          <a href={`/boards/${board.slug}`} class="board-card">
            <span class="name">{board.name}</span>
            <span class="meta">/{board.slug} · {board.role}</span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }
  header h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
  }
  .lead {
    color: var(--color-fg-muted);
  }
  .create {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: var(--space-3);
    align-items: end;
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .field {
    display: grid;
    gap: var(--space-1);
  }
  .field > span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .field input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .primary {
    padding: var(--space-2) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    height: fit-content;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .error {
    grid-column: 1 / -1;
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
  .empty {
    color: var(--color-fg-muted);
    text-align: center;
    padding: var(--space-6);
  }
  .boards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-3);
  }
  .board-card {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    text-decoration: none;
    color: inherit;
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .board-card:hover {
    border-color: var(--color-accent);
  }
  .name {
    font-weight: 600;
  }
  .meta {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }

  @media (max-width: 640px) {
    .create {
      grid-template-columns: 1fr;
    }
  }
</style>

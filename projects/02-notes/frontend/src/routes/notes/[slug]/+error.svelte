<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{page.status} — Notes</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  <p class="status">{page.status}</p>
  <h1>{page.error?.message ?? 'Something went wrong'}</h1>
  <p class="hint">
    {#if page.status === 404}
      That note doesn't exist. It may have been deleted, or the URL has a typo.
    {:else}
      An unexpected error occurred loading this note.
    {/if}
  </p>
  <a class="primary" href="/">Back to all notes</a>
</main>

<style>
  main {
    max-width: 480px;
    margin-inline: auto;
    padding: var(--space-16) var(--space-4);
    text-align: center;
    display: grid;
    gap: var(--space-4);
  }

  .status {
    font-size: var(--text-4xl);
    font-weight: 800;
    color: var(--color-accent);
    line-height: 1;
  }

  h1 {
    font-size: var(--text-2xl);
    font-weight: 700;
  }

  .hint {
    color: var(--color-fg-muted);
  }

  .primary {
    justify-self: center;
    margin-top: var(--space-3);
    padding: var(--space-3) var(--space-5);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    text-decoration: none;
  }

  .primary:hover {
    background: var(--color-accent-hover);
  }
</style>

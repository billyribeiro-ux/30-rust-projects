<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{page.status} — Reading</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  <p class="status">{page.status}</p>
  <h1>{page.error?.message ?? 'Something went wrong'}</h1>
  <a class="primary" href="/">Back to all books</a>
</main>

<style>
  main {
    max-width: 480px;
    margin-inline: auto;
    padding: var(--space-16) var(--space-4);
    text-align: center;
    display: grid;
    gap: var(--space-3);
  }
  .status {
    font-size: var(--text-4xl);
    font-weight: 800;
    color: var(--color-accent);
  }
  h1 {
    font-size: var(--text-xl);
  }
  .primary {
    justify-self: center;
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
  }
</style>

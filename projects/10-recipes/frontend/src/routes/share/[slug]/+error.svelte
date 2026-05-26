<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>Share link invalid</title>
</svelte:head>

<main>
  <h1>Link unavailable</h1>
  <p>{page.error?.message ?? 'This share link is invalid or has expired.'}</p>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-12) var(--space-4);
    display: grid;
    gap: var(--space-3);
  }
  h1 {
    font-size: var(--text-2xl);
    font-weight: 800;
  }
</style>

<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{page.status} — Recipes</title>
</svelte:head>

<main>
  <h1>Recipe not found</h1>
  <p>The recipe you're looking for doesn't exist or was deleted.</p>
  <p><a href="/">Back to all recipes</a></p>
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

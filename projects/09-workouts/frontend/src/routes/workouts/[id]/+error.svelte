<script lang="ts">
  import { page } from '$app/state';
</script>

<main>
  <h1>Not found</h1>
  <p>{page.error?.message ?? 'That workout does not exist.'}</p>
  <p><a href="/">Back to dashboard</a></p>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-12) var(--space-4);
    display: grid;
    gap: var(--space-3);
    text-align: center;
  }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
</style>

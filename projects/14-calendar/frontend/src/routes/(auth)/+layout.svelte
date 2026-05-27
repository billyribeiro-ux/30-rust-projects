<script lang="ts">
  let { children } = $props();
</script>

<div class="shell">
  <main>{@render children()}</main>
</div>

<style>
  .shell {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: var(--color-bg-sunken);
  }
  main {
    width: 100%;
    max-width: 420px;
    padding: var(--space-8) var(--space-6);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
  }
</style>

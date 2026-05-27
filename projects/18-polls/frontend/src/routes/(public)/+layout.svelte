<script lang="ts">
  let { children } = $props();
</script>

<main class="public">{@render children()}</main>

<style>
  .public {
    min-height: 100dvh;
    padding: var(--space-5);
    max-width: 36rem;
    margin: 0 auto;
  }
</style>

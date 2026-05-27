<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<svelte:head>
  <title>Storefront — Digital downloads</title>
  <meta
    name="description"
    content="Buy digital downloads — pay with Stripe, get a signed delivery link by email."
  />
</svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>
<div id="main">{@render children()}</div>

<style>
  .skip-link {
    position: absolute;
    left: var(--space-2);
    top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-sm);
    transform: translateY(-150%);
    transition: transform var(--duration-fast) var(--ease-out);
    z-index: var(--z-overlay);
  }
  .skip-link:focus-visible {
    transform: translateY(0);
  }
</style>

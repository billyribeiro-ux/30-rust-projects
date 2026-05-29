<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<svelte:head>
  <title>Newsletter — paid posts with Stripe Subscriptions</title>
  <meta
    name="description"
    content="Self-hosted newsletter. Free + Pro tiers via Stripe Subscriptions, magic-link auth, RSS feed, gated posts with paywall."
  />
  <link rel="alternate" type="application/rss+xml" title="RSS" href={`${(import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3020'}/feed.xml`} />
</svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>
<div id="main">{@render children()}</div>

<style>
  .skip-link {
    position: absolute; left: var(--space-2); top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-accent); color: var(--color-accent-fg);
    border-radius: var(--radius-sm); transform: translateY(-150%);
    transition: transform var(--duration-fast) var(--ease-out);
    z-index: var(--z-overlay);
  }
  .skip-link:focus-visible { transform: translateY(0); }
</style>

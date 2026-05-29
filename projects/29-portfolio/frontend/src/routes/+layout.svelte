<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<svelte:head>
  <title>Cinematic Portfolio — single-author, mdsvex-like pipeline</title>
  <meta name="description" content="A cinematic portfolio with GSAP reveal animations, MDX-ish content, and view-transition route changes." />
</svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>
<div id="main">{@render children()}</div>

<style>
  .skip-link { position: absolute; left: var(--space-2); top: var(--space-2); padding: var(--space-2) var(--space-3); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-sm); transform: translateY(-150%); transition: transform var(--duration-fast) var(--ease-out); z-index: var(--z-overlay); }
  .skip-link:focus-visible { transform: translateY(0); }

  :global(html) { color-scheme: light dark; }
  :global(body) { background: var(--color-bg); color: var(--color-fg); }

  /* View Transitions API for cross-route hero animation. The
     `<a>`s with view-transition-name on the hero make navigation
     feel like the image is the same node, not a re-render. */
  :global(html) { view-transition-name: root; }
  @media (prefers-reduced-motion: reduce) {
    :global(*) { animation-duration: 0.001ms !important; transition-duration: 0.001ms !important; }
  }
</style>

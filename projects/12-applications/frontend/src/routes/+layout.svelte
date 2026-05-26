<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

<svelte:head>
  <title>Applications — Job application tracker</title>
  <meta name="description" content="Track job applications, status timeline, next steps with due-date reminders, ICS calendar export." />
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
  .skip-link:focus-visible { transform: translateY(0); }
</style>

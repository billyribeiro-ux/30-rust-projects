<script lang="ts">
  import '../app.css';
  import { Notepad } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';

  let { children } = $props();
</script>

<svelte:head>
  <title>Notes — Markdown drafts that won't get lost</title>
  <meta
    name="description"
    content="A tiny, fast Markdown notes app. Draft, sanitize, share. Your ideas, in one place."
  />
  <link rel="canonical" href="https://notes.example.com/" />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="Notes — Markdown drafts that won't get lost" />
  <meta property="og:description" content="A tiny, fast Markdown notes app. Draft, sanitize, share." />
  <meta name="twitter:card" content="summary" />
</svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/">
    <Icon icon={Notepad} size={22} weight="duotone" />
    <span>Notes</span>
  </a>
  <ul>
    <li><a href="/">All notes</a></li>
    <li><a href="/notes/new">New</a></li>
    <li><a href="/about">About</a></li>
  </ul>
</nav>

<div id="main">
  {@render children()}
</div>

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

  .topnav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-elev);
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
    font-weight: 700;
    text-decoration: none;
  }

  .topnav ul {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .topnav ul a {
    color: var(--color-fg-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }

  .topnav ul a:hover,
  .topnav ul a:focus-visible {
    color: var(--color-fg);
  }

  @media (min-width: 768px) {
    .topnav {
      padding: var(--space-4) var(--space-8);
    }

    .topnav ul {
      gap: var(--space-6);
    }
  }
</style>

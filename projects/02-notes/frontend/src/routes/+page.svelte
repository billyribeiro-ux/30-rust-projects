<script lang="ts">
  import { fade } from 'svelte/transition';
  import { Plus } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import NoteCard from '$lib/components/NoteCard.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const notes = $derived(data.notes);
</script>

<svelte:head>
  <title>All notes — Notes</title>
  <meta
    name="description"
    content="Your Markdown notes — drafts, journals, ideas. {notes.length} {notes.length === 1 ? 'note' : 'notes'} so far."
  />
</svelte:head>

<main>
  <header>
    <h1>Your notes</h1>
    <a class="primary" href="/notes/new">
      <Icon icon={Plus} size={18} weight="bold" />
      <span>New note</span>
    </a>
  </header>

  {#if notes.length === 0}
    <p class="empty">
      No notes yet. <a href="/notes/new">Start your first one</a>.
    </p>
  {:else}
    <ul class="grid">
      {#each notes as note (note.id)}
        <li in:fade={{ duration: 180 }}>
          <NoteCard {note} />
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    max-width: var(--container-lg);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  h1 {
    font-size: var(--text-3xl);
    font-weight: 700;
    line-height: var(--leading-tight);
  }

  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    text-decoration: none;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .primary:hover {
    background: var(--color-accent-hover);
  }

  .empty {
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }

  .grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-4);
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
      gap: var(--space-8);
    }

    h1 {
      font-size: var(--text-4xl);
    }

    .grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (min-width: 1024px) {
    .grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>

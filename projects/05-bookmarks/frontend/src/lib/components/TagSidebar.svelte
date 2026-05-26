<script lang="ts">
  import { X } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import type { TagWithCount } from '$lib/types';

  type Props = {
    tags: TagWithCount[];
    activeTag: string;
    onSelect: (tag: string) => void;
  };

  let { tags, activeTag, onSelect }: Props = $props();
</script>

<nav class="sidebar" aria-label="Tag filter">
  <h2>Tags</h2>
  {#if activeTag}
    <button type="button" class="clear" onclick={() => onSelect('')}>
      <Icon icon={X} size={12} weight="bold" />
      <span>Clear filter ({activeTag})</span>
    </button>
  {/if}
  {#if tags.length === 0}
    <p class="empty">No tags yet.</p>
  {:else}
    <ul>
      {#each tags as t (t.name)}
        <li>
          <button
            type="button"
            class="tag"
            class:active={t.name === activeTag}
            aria-pressed={t.name === activeTag}
            onclick={() => onSelect(t.name === activeTag ? '' : t.name)}
          >
            <span class="name">{t.name}</span>
            <span class="count">{t.count}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</nav>

<style>
  .sidebar {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    height: fit-content;
    position: sticky;
    top: var(--space-4);
  }

  h2 {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .clear {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-align: left;
  }

  .clear:hover {
    color: var(--color-fg);
  }

  .empty {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 2px;
  }

  .tag {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    color: var(--color-fg);
    text-align: left;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .tag:hover {
    background: var(--color-bg-sunken);
  }

  .tag.active {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }

  .name {
    font-size: var(--text-sm);
  }

  .count {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .tag.active .count {
    color: var(--color-accent-fg);
    opacity: 0.85;
  }
</style>

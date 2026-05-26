<script lang="ts">
  import { Trash, ArrowSquareOut, Tag as TagIcon } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import type { Bookmark } from '$lib/types';

  type Props = {
    bookmark: Bookmark;
    activeTag: string;
    onDelete: (id: string) => void;
    onTagClick: (tag: string) => void;
  };

  let { bookmark, activeTag, onDelete, onTagClick }: Props = $props();

  const host = $derived(safeHost(bookmark.url));

  function safeHost(url: string): string {
    try {
      return new URL(url).host;
    } catch {
      return url;
    }
  }
</script>

<article class="card">
  <header>
    <a
      class="title"
      href={bookmark.url}
      target="_blank"
      rel="noopener noreferrer"
      data-sveltekit-preload-data="off"
    >
      <span>{bookmark.title}</span>
      <Icon icon={ArrowSquareOut} size={14} />
    </a>
    <p class="host">{host}</p>
  </header>

  {#if bookmark.description}
    <p class="description">{bookmark.description}</p>
  {/if}

  {#if bookmark.tags.length > 0}
    <ul class="tags" aria-label="Tags">
      {#each bookmark.tags as tag (tag)}
        <li>
          <button
            type="button"
            class="tag"
            class:active={tag === activeTag}
            onclick={() => onTagClick(tag)}
            aria-pressed={tag === activeTag}
            aria-label={`Filter by tag ${tag}`}
          >
            <Icon icon={TagIcon} size={12} />
            <span>{tag}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <button
    type="button"
    class="delete"
    aria-label={`Delete bookmark "${bookmark.title}"`}
    onclick={() => {
      if (confirm(`Delete "${bookmark.title}"?`)) onDelete(bookmark.id);
    }}
  >
    <Icon icon={Trash} size={16} />
  </button>
</article>

<style>
  .card {
    position: relative;
    padding: var(--space-4) var(--space-5);
    padding-right: var(--space-12);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-2);
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  header {
    display: grid;
    gap: 2px;
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
    font-size: var(--text-lg);
    color: var(--color-fg);
    line-height: var(--leading-tight);
  }

  .title:hover {
    color: var(--color-accent);
  }

  .host {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    font-family: var(--font-mono);
  }

  .description {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-2);
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border: 1px solid transparent;
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: 500;
    transition: border-color var(--duration-fast) var(--ease-out);
  }

  .tag:hover {
    border-color: var(--color-border-strong);
  }

  .tag.active {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }

  .delete {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    width: 32px;
    height: 32px;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .delete:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }
</style>

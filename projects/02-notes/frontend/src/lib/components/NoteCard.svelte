<script lang="ts">
  import type { NoteSummary } from '$lib/types';

  type Props = { note: NoteSummary };
  let { note }: Props = $props();

  const updated = $derived(formatDate(note.updated_at));

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }
</script>

<article class="card">
  <a href="/notes/{note.slug}" class="link">
    <h2>{note.title}</h2>
    <p class="excerpt">{note.excerpt}</p>
    <time class="updated" datetime={note.updated_at}>{updated}</time>
  </a>
</article>

<style>
  .card {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    transition:
      border-color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out);
  }

  .card:hover {
    border-color: var(--color-border-strong);
  }

  .link {
    display: block;
    padding: var(--space-4) var(--space-5);
    color: inherit;
    text-decoration: none;
  }

  h2 {
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: var(--leading-tight);
    margin-bottom: var(--space-2);
  }

  .excerpt {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .updated {
    display: block;
    margin-top: var(--space-3);
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
  }

  @media (min-width: 768px) {
    .link {
      padding: var(--space-5) var(--space-6);
    }

    h2 {
      font-size: var(--text-xl);
    }
  }
</style>

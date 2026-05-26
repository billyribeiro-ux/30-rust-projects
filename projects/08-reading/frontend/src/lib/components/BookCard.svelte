<script lang="ts">
  import type { Book } from '$lib/types';
  import { STATUS_LABEL } from '$lib/types';

  type Props = { book: Book };
  let { book }: Props = $props();

  const progress = $derived(
    book.pages && book.pages > 0
      ? Math.min(100, Math.round((book.current_page / book.pages) * 100))
      : 0
  );
</script>

<article class="card" class:reading={book.status === 'reading'}>
  <a href="/books/{book.id}" class="link">
    <div class="cover">
      {#if book.cover_url}
        <img src={book.cover_url} alt="" loading="lazy" />
      {:else}
        <div class="cover-fallback" aria-hidden="true">
          <span>{book.title.slice(0, 2).toUpperCase()}</span>
        </div>
      {/if}
    </div>
    <div class="meta">
      <h2>{book.title}</h2>
      {#if book.author}<p class="author">{book.author}</p>{/if}
      <p class="status">{STATUS_LABEL[book.status]}</p>
      {#if book.status === 'reading' && book.pages}
        <div
          class="progress"
          role="progressbar"
          aria-valuenow={progress}
          aria-valuemin="0"
          aria-valuemax="100"
          aria-label={`Reading progress for ${book.title}`}
        >
          <div class="bar" style="--value: {progress}%"></div>
          <p class="progress-label">{book.current_page} / {book.pages} pages ({progress}%)</p>
        </div>
      {/if}
    </div>
  </a>
</article>

<style>
  .card {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .card:hover {
    border-color: var(--color-border-strong);
  }
  .card.reading {
    border-color: var(--color-accent);
  }
  .link {
    display: grid;
    grid-template-columns: 80px 1fr;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    color: inherit;
    text-decoration: none;
  }
  .cover {
    width: 80px;
    aspect-ratio: 2 / 3;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--color-bg-sunken);
    color: var(--color-fg-muted);
    font-weight: 700;
    font-size: var(--text-lg);
  }
  .meta {
    display: grid;
    gap: 2px;
    align-content: start;
  }
  h2 {
    font-size: var(--text-base);
    font-weight: 600;
    line-height: var(--leading-tight);
  }
  .author {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .status {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-top: var(--space-1);
  }
  .progress {
    margin-top: var(--space-2);
  }
  .bar {
    height: 4px;
    background: var(--color-bg-sunken);
    border-radius: 999px;
    overflow: hidden;
    position: relative;
  }
  .bar::after {
    content: '';
    position: absolute;
    inset: 0;
    width: var(--value);
    background: var(--color-accent);
  }
  .progress-label {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    margin-top: 2px;
    font-variant-numeric: tabular-nums;
  }
</style>

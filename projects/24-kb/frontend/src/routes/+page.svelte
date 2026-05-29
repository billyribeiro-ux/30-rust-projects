<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  let q = $state(data.q);
</script>

<svelte:head>
  <title>Knowledge Base{data.q ? ` — ${data.q}` : ''}</title>
</svelte:head>

<main>
  <header class="hero">
    <h1>Knowledge Base</h1>
    <p class="lead">Hybrid search with Postgres FTS, typo-tolerant <code>pg_trgm</code> fallback, and optional Meili re-ranking.</p>
    <form method="GET" role="search">
      <input
        type="search"
        name="q"
        placeholder="Search articles…"
        bind:value={q}
        aria-label="Search query"
      />
      <button type="submit" class="primary">Search</button>
    </form>
  </header>

  {#if data.q}
    <section aria-labelledby="results-h">
      <h2 id="results-h">Results for “{data.q}”</h2>
      {#if data.hits.length === 0}
        <p class="empty">No matches. Try different words or check spelling.</p>
      {:else}
        <ul class="results">
          {#each data.hits as hit (hit.id)}
            <li>
              <a class="card" href={`/a/${hit.slug}`}>
                <span class="title">{hit.title}</span>
                <span class="snippet">{@html hit.snippet}</span>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  <section aria-labelledby="articles-h">
    <h2 id="articles-h">Articles</h2>
    {#if data.articles.length === 0}
      <p class="empty">No articles yet.</p>
    {:else}
      <ul>
        {#each data.articles as a (a.id)}
          <li>
            <a class="card" href={`/a/${a.slug}`}>
              <span class="title">{a.title}</span>
              <span class="summary">{a.summary}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  main { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-6); }
  .hero h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin: var(--space-2) 0 var(--space-4); }
  .lead code { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
  form { display: grid; grid-template-columns: 1fr auto; gap: var(--space-2); }
  input[type="search"] { padding: var(--space-3) var(--space-4); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-3) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  h2 { font-size: var(--text-xl); font-weight: 700; margin-bottom: var(--space-3); }
  .empty { color: var(--color-fg-muted); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .card { display: grid; gap: var(--space-1); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; }
  .title { font-weight: 600; }
  .summary, .snippet { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .snippet :global(mark) { background: hsl(45 100% 50% / 0.3); color: inherit; padding: 0 2px; border-radius: 2px; }
</style>

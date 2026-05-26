<script lang="ts">
  import { Trash, PencilSimple } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const note = $derived(data.note);

  const updated = $derived(formatDate(note.updated_at));

  const jsonld = $derived({
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: note.title,
    description: note.excerpt,
    dateModified: note.updated_at,
    datePublished: note.created_at
  });

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' });
  }
</script>

<svelte:head>
  <title>{note.title} — Notes</title>
  <meta name="description" content={note.excerpt} />
  <link rel="canonical" href={`https://notes.example.com/notes/${note.slug}`} />
  <meta property="og:type" content="article" />
  <meta property="og:title" content={note.title} />
  <meta property="og:description" content={note.excerpt} />
  <meta name="twitter:card" content="summary" />
  {@html `<script type="application/ld+json">${JSON.stringify(jsonld)}</` + `script>`}
</svelte:head>

<svelte:boundary onerror={(e) => console.error('note render boundary', e)}>
  <main>
    <article>
      <header>
        <h1>{note.title}</h1>
        <time class="updated" datetime={note.updated_at}>Updated {updated}</time>
      </header>

      <div class="prose">{@html note.body_html}</div>
    </article>

    <aside class="actions" aria-label="Note actions">
      <a class="ghost" href={`/notes/${note.slug}/edit`}>
        <Icon icon={PencilSimple} size={16} weight="bold" />
        <span>Edit</span>
      </a>

      <form
        method="POST"
        action="?/remove"
        onsubmit={(e) => {
          if (!confirm(`Delete "${note.title}"? This cannot be undone.`)) e.preventDefault();
        }}
      >
        <button type="submit" class="danger">
          <Icon icon={Trash} size={16} weight="bold" />
          <span>Delete</span>
        </button>
      </form>
    </aside>
  </main>

  {#snippet failed(err, reset)}
    <main>
      <div class="boundary-error" role="alert">
        <h2>Something went wrong rendering this note.</h2>
        <p>{(err as Error).message}</p>
        <button type="button" onclick={reset}>Try again</button>
      </div>
    </main>
  {/snippet}
</svelte:boundary>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }

  article header {
    margin-bottom: var(--space-6);
    padding-bottom: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
    line-height: var(--leading-tight);
  }

  .updated {
    display: block;
    margin-top: var(--space-2);
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }

  .prose {
    line-height: var(--leading-relaxed);
  }

  .prose :global(h1),
  .prose :global(h2),
  .prose :global(h3) {
    margin: var(--space-6) 0 var(--space-3);
    line-height: var(--leading-tight);
    font-weight: 700;
  }

  .prose :global(h1) {
    font-size: var(--text-2xl);
  }
  .prose :global(h2) {
    font-size: var(--text-xl);
  }
  .prose :global(h3) {
    font-size: var(--text-lg);
  }

  .prose :global(p) {
    margin-bottom: var(--space-4);
  }

  .prose :global(ul),
  .prose :global(ol) {
    padding-left: 1.5em;
    margin-bottom: var(--space-4);
  }

  .prose :global(li) {
    margin-bottom: var(--space-1);
  }

  .prose :global(code) {
    font-family: var(--font-mono);
    background: var(--color-bg-sunken);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    font-size: 0.9em;
  }

  .prose :global(pre) {
    padding: var(--space-4);
    margin-bottom: var(--space-4);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    overflow-x: auto;
  }

  .prose :global(blockquote) {
    border-left: 3px solid var(--color-accent);
    padding-left: var(--space-4);
    margin-bottom: var(--space-4);
    color: var(--color-fg-muted);
  }

  .actions {
    display: flex;
    gap: var(--space-3);
    justify-content: flex-end;
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-border);
  }

  .ghost,
  .danger {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--text-sm);
    text-decoration: none;
  }

  .ghost {
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    background: var(--color-bg-elev);
  }

  .ghost:hover {
    border-color: var(--color-border-strong);
  }

  .danger {
    color: hsl(0 72% 42%);
    border: 1px solid hsl(0 72% 51% / 0.4);
    background: var(--color-bg);
  }

  .danger:hover {
    background: hsl(0 72% 51% / 0.08);
  }

  .boundary-error {
    padding: var(--space-6);
    background: hsl(0 72% 51% / 0.06);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }

  .boundary-error button {
    justify-self: start;
    padding: var(--space-2) var(--space-3);
    color: var(--color-accent);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }

    h1 {
      font-size: var(--text-4xl);
    }
  }
</style>

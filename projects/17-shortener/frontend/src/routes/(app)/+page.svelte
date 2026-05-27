<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  function shortUrl(slug: string): string {
    return `${data.publicBase}/${slug}`;
  }

  async function copy(slug: string) {
    try {
      await navigator.clipboard.writeText(shortUrl(slug));
    } catch {
      /* ignore */
    }
  }
</script>

<svelte:head><title>Your short links</title></svelte:head>

<main>
  <header>
    <h1>Your short links</h1>
    <p class="lead">Shorten, share, and track clicks across your campaigns.</p>
  </header>

  <section class="panel" aria-label="Create a new short link">
    <h2>New link</h2>
    <form method="POST" action="?/newLink" use:enhance>
      <label class="field">
        <span>Target URL <em>*</em></span>
        <input
          type="url"
          name="target_url"
          required
          placeholder="https://example.com/some/long/path"
          value={(form?.target_url as string | undefined) ?? ''}
        />
      </label>
      <label class="field">
        <span>Custom slug (optional)</span>
        <input
          type="text"
          name="slug"
          pattern={'[a-zA-Z0-9_-]{3,32}'}
          placeholder="my-campaign"
          value={(form?.slug as string | undefined) ?? ''}
        />
      </label>
      {#if form && 'error' in form && form.error}
        <p class="error" role="alert">{form.error}</p>
      {/if}
      <button type="submit" class="primary">Create short link</button>
    </form>
  </section>

  {#if data.links.length === 0}
    <p class="empty">No links yet. Create your first one above.</p>
  {:else}
    <ul class="links">
      {#each data.links as link (link.id)}
        <li class="link">
          <div class="link-main">
            <a class="slug" href={shortUrl(link.slug)} target="_blank" rel="noopener noreferrer">
              /{link.slug}
            </a>
            <span class="target" title={link.target_url}>{link.target_url}</span>
          </div>
          <div class="link-meta">
            <span class="counter" aria-label="click count">
              {link.click_count ?? 0} clicks
            </span>
            <button type="button" class="ghost" onclick={() => copy(link.slug)}>Copy</button>
            <a class="ghost" href={`/links/${encodeURIComponent(link.slug)}/stats`}>Stats</a>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    max-width: var(--container-md, 960px);
    margin-inline: auto;
    padding: var(--space-5) var(--space-4);
    display: grid;
    gap: var(--space-4);
  }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  .lead { color: var(--color-fg-muted); }
  .panel {
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  form { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field em { color: var(--color-danger); font-style: normal; }
  .field input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  .primary {
    padding: var(--space-2) var(--space-3);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-sm);
    font-weight: 600;
    justify-self: start;
  }
  .ghost {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    text-decoration: none;
  }
  .error {
    padding: var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-sm);
  }
  .empty {
    padding: var(--space-3);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    color: var(--color-fg-muted);
    text-align: center;
  }
  .links {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-2);
  }
  .link {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-3);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .link-main {
    display: grid;
    gap: 2px;
    flex: 1;
    min-width: 200px;
  }
  .slug {
    font-weight: 700;
    color: var(--color-accent);
    text-decoration: none;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .target {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 50ch;
  }
  .link-meta { display: flex; gap: var(--space-2); align-items: center; }
  .counter {
    font-variant-numeric: tabular-nums;
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
</style>

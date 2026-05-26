<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const c = $derived(data.contact);
</script>

<svelte:head><title>{c.name} — Contacts</title></svelte:head>

<main>
  <header>
    <a class="back" href="/contacts">← Back</a>
  </header>
  <article>
    <h1>{c.name}</h1>
    {#if c.company}<p class="company">{c.company}</p>{/if}
    <dl class="fields">
      {#if c.email}<div><dt>Email</dt><dd><a href="mailto:{c.email}">{c.email}</a></dd></div>{/if}
      {#if c.phone}<div><dt>Phone</dt><dd>{c.phone}</dd></div>{/if}
      {#if c.last_contacted_at}
        <div>
          <dt>Last contacted</dt>
          <dd><time datetime={c.last_contacted_at}>{new Date(c.last_contacted_at).toLocaleString()}</time></dd>
        </div>
      {/if}
    </dl>

    {#if c.notes}
      <section class="notes">
        <h2>Notes</h2>
        <p>{c.notes}</p>
      </section>
    {/if}

    {#if c.tags.length > 0}
      <section class="tags-section" aria-label="Tags">
        <h2>Tags</h2>
        <ul class="tags">
          {#each c.tags as t (t)}
            <li><a href="/contacts?tag={encodeURIComponent(t)}">{t}</a></li>
          {/each}
        </ul>
      </section>
    {/if}

    <div class="actions">
      <form method="POST" action="?/touch" use:enhance>
        <button type="submit" class="ghost">Mark as contacted now</button>
      </form>
      <form
        method="POST"
        action="?/remove"
        onsubmit={(e) => {
          if (!confirm(`Delete ${c.name}? This is a soft delete; you can restore later.`)) e.preventDefault();
        }}
      >
        <button type="submit" class="danger">Delete</button>
      </form>
    </div>
  </article>
</main>

<style>
  main { max-width: var(--container-md); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  .back { color: var(--color-fg-muted); font-size: var(--text-sm); }
  article { padding: var(--space-6); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: grid; gap: var(--space-4); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .company { color: var(--color-fg-muted); }
  .fields { display: grid; gap: var(--space-2); margin: 0; }
  .fields > div { display: grid; grid-template-columns: 120px 1fr; gap: var(--space-3); }
  dt { color: var(--color-fg-muted); font-size: var(--text-sm); }
  dd { font-size: var(--text-sm); }
  .notes h2, .tags-section h2 { font-size: var(--text-sm); color: var(--color-fg-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: var(--space-2); }
  .notes p { white-space: pre-wrap; line-height: var(--leading-relaxed); }
  .tags { list-style: none; padding: 0; margin: 0; display: flex; flex-wrap: wrap; gap: var(--space-1); }
  .tags li a { font-size: var(--text-xs); padding: 2px var(--space-2); background: var(--color-bg-sunken); border-radius: var(--radius-full); color: var(--color-fg-muted); }
  .actions { display: flex; gap: var(--space-3); padding-top: var(--space-3); border-top: 1px solid var(--color-border); }
  .ghost, .danger { padding: var(--space-2) var(--space-4); border-radius: var(--radius-md); font-size: var(--text-sm); font-weight: 600; }
  .ghost { color: var(--color-fg); border: 1px solid var(--color-border); background: var(--color-bg); }
  .ghost:hover { border-color: var(--color-border-strong); }
  .danger { color: hsl(0 72% 42%); border: 1px solid hsl(0 72% 51% / 0.4); background: var(--color-bg); }
  .danger:hover { background: hsl(0 72% 51% / 0.08); }
</style>

<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const d = $derived(data.dashboard);
</script>

<svelte:head><title>Dashboard — Contacts</title></svelte:head>

<main>
  <header>
    <h1>Dashboard</h1>
    <p class="lead">Hello, {data.user?.name || data.user?.email}.</p>
  </header>

  <dl class="kpis" aria-label="Key totals">
    <div>
      <dt>Contacts</dt>
      <dd>{d.total_contacts}</dd>
    </div>
    <div>
      <dt>Stale (7d+)</dt>
      <dd>{d.stale_contacts_7d}</dd>
    </div>
    <div>
      <dt>Touchpoints (7d)</dt>
      <dd>{d.interactions_this_week}</dd>
    </div>
    <div>
      <dt>Trashed</dt>
      <dd>{d.deleted_contacts}</dd>
    </div>
  </dl>

  <section class="grid">
    <article class="card" aria-label="Follow-ups due">
      <h2>Follow-ups due</h2>
      {#if d.due_reminders.length === 0}
        <p class="empty">Nothing due in the next 7 days.</p>
      {:else}
        <ul>
          {#each d.due_reminders as r (r.id)}
            <li>
              <a href="/contacts/{r.contact_id}">{r.contact_name}</a> — {r.body}
              <time datetime={r.due_at}>{new Date(r.due_at).toLocaleDateString()}</time>
            </li>
          {/each}
        </ul>
      {/if}
    </article>

    <article class="card" aria-label="Recent contacts">
      <h2>Recently contacted</h2>
      {#if d.recent_contacts.length === 0}
        <p class="empty">No contacts yet. <a href="/contacts/new">Add one</a>.</p>
      {:else}
        <ul>
          {#each d.recent_contacts as c (c.id)}
            <li>
              <a href="/contacts/{c.id}">{c.name}</a>
              {#if c.company}<span class="muted"> — {c.company}</span>{/if}
              {#if c.last_contacted_at}
                <time datetime={c.last_contacted_at}>
                  {new Date(c.last_contacted_at).toLocaleDateString()}
                </time>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </article>
  </section>
</main>

<style>
  main { max-width: var(--container-lg); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-6); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .kpis { display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--space-3); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .kpis > div { display: grid; gap: 2px; }
  .kpis dt { color: var(--color-fg-muted); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; }
  .kpis dd { font-size: var(--text-2xl); font-weight: 700; font-variant-numeric: tabular-nums; }
  .grid { display: grid; gap: var(--space-4); }
  .card { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  h2 { font-size: var(--text-lg); font-weight: 700; margin-bottom: var(--space-3); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  li { display: flex; gap: var(--space-2); justify-content: space-between; align-items: baseline; padding: var(--space-2) 0; border-bottom: 1px solid var(--color-border); font-size: var(--text-sm); }
  li:last-child { border-bottom: 0; }
  time { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .muted { color: var(--color-fg-muted); }
  .empty { color: var(--color-fg-muted); padding: var(--space-3); background: var(--color-bg-sunken); border-radius: var(--radius-sm); text-align: center; }

  @media (min-width: 768px) {
    .kpis { grid-template-columns: repeat(4, 1fr); }
    .grid { grid-template-columns: 1fr 1fr; }
  }
</style>

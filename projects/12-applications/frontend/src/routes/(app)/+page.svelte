<script lang="ts">
  import type { PageProps } from './$types';
  import { STATUS_LABEL } from '$lib/types';

  let { data }: PageProps = $props();
  const d = $derived(data.dashboard);
</script>

<svelte:head><title>Dashboard — Applications</title></svelte:head>

<main>
  <header>
    <h1>Dashboard</h1>
    <p class="lead">Hi, {data.user?.name || data.user?.email}.</p>
  </header>

  <dl class="kpis" aria-label="Totals">
    <div>
      <dt>Total</dt>
      <dd>{d.total_applications}</dd>
    </div>
    <div>
      <dt>Active pipeline</dt>
      <dd>{d.active_pipeline}</dd>
    </div>
    <div>
      <dt>Due in 7 days</dt>
      <dd>{d.due_this_week.length}</dd>
    </div>
  </dl>

  <section class="panel" aria-label="By status">
    <h2>By status</h2>
    {#if d.by_status.length === 0}
      <p class="empty">No applications yet. <a href="/applications/new">Add one</a>.</p>
    {:else}
      <ul class="status-bars">
        {#each d.by_status as s (s.status)}
          <li>
            <span class="status-label">{STATUS_LABEL[s.status]}</span>
            <span class="status-count">{s.count}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel" aria-label="Due this week">
    <h2>Due this week</h2>
    {#if d.due_this_week.length === 0}
      <p class="empty">Nothing due.</p>
    {:else}
      <ul>
        {#each d.due_this_week as s (s.id)}
          <li>
            <a href="/applications/{s.application_id}">{s.company}</a> — {s.body}
            <time datetime={s.due_at}>{new Date(s.due_at).toLocaleDateString()}</time>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="hint">
      <a href="/api/export/next-steps.ics">Subscribe via ICS</a>
      — add this URL to Apple Calendar / Google Calendar for due-date reminders.
    </p>
  </section>

  <section class="panel" aria-label="Recent events">
    <h2>Recent activity</h2>
    {#if d.recent_events.length === 0}
      <p class="empty">No activity yet.</p>
    {:else}
      <ul>
        {#each d.recent_events as e (e.application_id + e.occurred_at)}
          <li>
            <a href="/applications/{e.application_id}">{e.company} — {e.role}</a>
            <span class="kind">{e.kind === 'status_change' ? `→ ${e.new_status}` : e.body}</span>
            <time datetime={e.occurred_at}>{new Date(e.occurred_at).toLocaleDateString()}</time>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  main { max-width: var(--container-lg); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .kpis { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .kpis > div { display: grid; gap: 2px; text-align: center; }
  .kpis dt { color: var(--color-fg-muted); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; }
  .kpis dd { font-size: var(--text-2xl); font-weight: 700; line-height: 1; }
  .panel { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: grid; gap: var(--space-3); }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  .status-bars { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .status-bars li { display: flex; justify-content: space-between; padding: var(--space-2) var(--space-3); background: var(--color-bg); border-radius: var(--radius-sm); }
  .status-count { font-weight: 600; font-variant-numeric: tabular-nums; }
  ul:not(.status-bars) { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  ul:not(.status-bars) li { display: flex; gap: var(--space-2); justify-content: space-between; align-items: baseline; padding: var(--space-2) 0; border-bottom: 1px solid var(--color-border); font-size: var(--text-sm); }
  ul:not(.status-bars) li:last-child { border-bottom: 0; }
  time { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .kind { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .empty { color: var(--color-fg-muted); padding: var(--space-3); background: var(--color-bg-sunken); border-radius: var(--radius-sm); text-align: center; }
  .hint { color: var(--color-fg-muted); font-size: var(--text-xs); margin-top: var(--space-2); }
</style>

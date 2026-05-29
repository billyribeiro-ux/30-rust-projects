<script lang="ts">
  import { kpisApi, ingestApi } from '$lib/api';
  import { rollNumber } from '$lib/numberRoll';
  import type { KpiSnapshot } from '$lib/types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  let kpis = $state<KpiSnapshot>(data.kpis);
  let wsStatus = $state<'connecting' | 'open' | 'closed'>('connecting');
  // svelte-ignore state_referenced_locally
  let prev = $state({
    // svelte-ignore state_referenced_locally
    events_last_minute: data.kpis.events_last_minute,
    // svelte-ignore state_referenced_locally
    events_last_hour: data.kpis.events_last_hour,
    // svelte-ignore state_referenced_locally
    unique_sessions_last_hour: data.kpis.unique_sessions_last_hour
  });

  // Refs to the number elements so the animator can manipulate textContent.
  let elMin = $state<HTMLElement | null>(null);
  let elHour = $state<HTMLElement | null>(null);
  let elSess = $state<HTMLElement | null>(null);

  $effect(() => {
    if (typeof EventSource === 'undefined') return;
    const es = new EventSource(kpisApi.streamUrl(), { withCredentials: true });
    es.onopen = () => (wsStatus = 'open');
    es.onerror = () => (wsStatus = 'closed');
    es.addEventListener('kpis', (ev) => {
      try {
        const next = JSON.parse((ev as MessageEvent).data) as KpiSnapshot;
        const before = { ...prev };
        prev = {
          events_last_minute: next.events_last_minute,
          events_last_hour: next.events_last_hour,
          unique_sessions_last_hour: next.unique_sessions_last_hour
        };
        kpis = next;
        if (elMin) rollNumber(elMin, before.events_last_minute, next.events_last_minute);
        if (elHour) rollNumber(elHour, before.events_last_hour, next.events_last_hour);
        if (elSess)
          rollNumber(elSess, before.unique_sessions_last_hour, next.unique_sessions_last_hour);
      } catch {
        /* ignore */
      }
    });
    return () => es.close();
  });

  async function emit() {
    try {
      await ingestApi.send(fetch, 'demo_click', { source: 'dashboard' }, 'dashboard-session');
    } catch {
      /* ignore */
    }
  }
</script>

<svelte:head><title>Dashboard — Realtime Analytics</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/dashboard">Analytics</a>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <span class={`badge status-${wsStatus}`}>SSE {wsStatus}</span>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <h1>Live KPIs</h1>
  <p class="lead">Postgres-aggregated KPIs broadcast over SSE every second. GSAP rolls the numbers.</p>

  <section class="kpis" aria-label="KPI cards">
    <article class="card">
      <h2>Events / minute</h2>
      <p class="value" bind:this={elMin}>{kpis.events_last_minute.toLocaleString()}</p>
    </article>
    <article class="card">
      <h2>Events / hour</h2>
      <p class="value" bind:this={elHour}>{kpis.events_last_hour.toLocaleString()}</p>
    </article>
    <article class="card">
      <h2>Unique sessions / hr</h2>
      <p class="value" bind:this={elSess}>{kpis.unique_sessions_last_hour.toLocaleString()}</p>
    </article>
  </section>

  <section aria-labelledby="top-h">
    <h2 id="top-h">Top event kinds (last hour)</h2>
    {#if kpis.top_kinds.length === 0}
      <p class="empty">No events yet — emit one below.</p>
    {:else}
      <ul>
        {#each kpis.top_kinds as t (t.kind)}
          <li><code>{t.kind}</code> <strong>{t.count.toLocaleString()}</strong></li>
        {/each}
      </ul>
    {/if}
  </section>

  <button type="button" class="primary" onclick={emit}>Emit demo event</button>
</main>

<style>
  .topnav { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .brand { font-weight: 800; color: var(--color-accent); flex: 1; }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; }
  .who { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .badge { padding: 0 var(--space-2); border-radius: var(--radius-pill); font-size: var(--text-xs); text-transform: uppercase; background: var(--color-bg-sunken); color: var(--color-fg-muted); }
  .status-open { background: hsl(140 60% 50% / 0.18); color: hsl(140 60% 25%); }
  .status-closed { background: hsl(0 72% 51% / 0.12); color: var(--color-danger); }
  main { max-width: 1080px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: var(--space-4); }
  .card { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .card h2 { font-size: var(--text-sm); text-transform: uppercase; color: var(--color-fg-muted); letter-spacing: 0.04em; font-weight: 700; }
  .value { font-size: var(--text-4xl); font-weight: 800; font-variant-numeric: tabular-nums; margin-top: var(--space-3); background: linear-gradient(135deg, var(--color-accent), hsl(220 70% 50%)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  ul li { display: flex; justify-content: space-between; padding: var(--space-2) var(--space-3); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  code { font-family: var(--font-mono, monospace); }
  .empty { color: var(--color-fg-muted); }
  .primary { padding: var(--space-3) var(--space-5); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; justify-self: start; }
</style>

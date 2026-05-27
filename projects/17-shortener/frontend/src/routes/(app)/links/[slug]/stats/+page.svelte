<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const s = $derived(data.stats);

  // Canvas-drawn bar chart, no chart-lib bloat (project-7 pattern).
  let canvas: HTMLCanvasElement | undefined = $state();
  $effect(() => {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const cssW = canvas.clientWidth;
    const cssH = 180;
    canvas.width = cssW * dpr;
    canvas.height = cssH * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, cssW, cssH);

    const buckets = s.daily;
    if (buckets.length === 0) {
      ctx.fillStyle = '#888';
      ctx.font = '14px system-ui';
      ctx.fillText('No clicks in the last 7 days', 12, 24);
      return;
    }
    const max = Math.max(1, ...buckets.map((b) => b.count));
    const padL = 36, padB = 24, padT = 12, padR = 8;
    const innerW = cssW - padL - padR;
    const innerH = cssH - padT - padB;
    const barW = innerW / buckets.length;

    // y-axis
    ctx.strokeStyle = '#ccc';
    ctx.beginPath();
    ctx.moveTo(padL, padT);
    ctx.lineTo(padL, padT + innerH);
    ctx.lineTo(padL + innerW, padT + innerH);
    ctx.stroke();
    ctx.fillStyle = '#666';
    ctx.font = '11px system-ui';
    ctx.fillText(String(max), 4, padT + 10);
    ctx.fillText('0', 22, padT + innerH);

    buckets.forEach((b, i) => {
      const h = (b.count / max) * innerH;
      const x = padL + i * barW + 2;
      const y = padT + innerH - h;
      ctx.fillStyle = '#3b82f6';
      ctx.fillRect(x, y, barW - 4, h);
      ctx.fillStyle = '#666';
      const label = b.day.slice(5); // MM-DD
      ctx.fillText(label, x, padT + innerH + 14);
    });
  });
</script>

<svelte:head><title>Stats — /{s.link.slug}</title></svelte:head>

<main>
  <header>
    <p><a href="/">← All links</a></p>
    <h1>/{s.link.slug}</h1>
    <p class="target">
      <a href={s.link.target_url} target="_blank" rel="noopener noreferrer">{s.link.target_url}</a>
    </p>
  </header>

  <section class="counters" aria-label="Counters">
    <div class="card">
      <span class="big">{s.total_clicks}</span>
      <span class="label">total clicks</span>
    </div>
    <div class="card">
      <span class="big">{s.clicks_today}</span>
      <span class="label">today</span>
    </div>
    <div class="card">
      <span class="big">{s.clicks_7d}</span>
      <span class="label">last 7d</span>
    </div>
    <div class="card">
      <span class="big">{s.clicks_30d}</span>
      <span class="label">last 30d</span>
    </div>
    {#if s.redis_counter !== null}
      <div class="card cache">
        <span class="big">{s.redis_counter}</span>
        <span class="label">redis live counter</span>
      </div>
    {/if}
  </section>

  <section class="panel" aria-label="Daily clicks">
    <h2>Last 7 days</h2>
    <canvas bind:this={canvas} aria-label="Daily click bar chart"></canvas>
  </section>

  <section class="panel">
    <h2>Top referers</h2>
    {#if s.top_referers.length === 0}
      <p class="muted">No clicks yet.</p>
    {:else}
      <ul class="rank">
        {#each s.top_referers as r (r.name)}
          <li><span>{r.name}</span><b>{r.count}</b></li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel">
    <h2>Top countries</h2>
    {#if s.top_countries.length === 0}
      <p class="muted">No clicks yet.</p>
    {:else}
      <ul class="rank">
        {#each s.top_countries as c (c.name)}
          <li><span>{c.name}</span><b>{c.count}</b></li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  main {
    max-width: var(--container-md, 960px);
    margin-inline: auto;
    padding: var(--space-5) var(--space-4);
    display: grid;
    gap: var(--space-4);
  }
  header { display: grid; gap: var(--space-1); }
  h1 { font-size: var(--text-3xl); font-weight: 800; font-family: var(--font-mono, ui-monospace, monospace); }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  .target a {
    color: var(--color-fg-muted);
    word-break: break-all;
    overflow-wrap: anywhere;
  }
  .counters {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: var(--space-2);
  }
  .card {
    padding: var(--space-3);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: 2px;
  }
  .card.cache { border-color: var(--color-accent); }
  .big { font-size: var(--text-2xl); font-weight: 800; font-variant-numeric: tabular-nums; }
  .label { font-size: var(--text-xs); color: var(--color-fg-muted); }
  .panel {
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  canvas { width: 100%; height: 180px; }
  .rank { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-1); }
  .rank li {
    display: flex;
    justify-content: space-between;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
  }
  .rank b { font-variant-numeric: tabular-nums; }
  .muted { color: var(--color-fg-muted); }
</style>

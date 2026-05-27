<script lang="ts">
  import { onMount } from 'svelte';
  import { Spring } from 'svelte/motion';
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import type { CountsEvent, OptionPublic } from '$lib/types';

  let { data }: PageProps = $props();

  // Mutable, live-updated copies of the initial server-rendered data.
  let isOpen = $state(data.poll.is_open);
  let counts = $state<OptionPublic[]>(data.poll.options.map((o) => ({ ...o })));
  let total = $state(data.poll.total_votes);

  // One spring per option id, keyed so re-ordering doesn't reset animations.
  // We store percentages (0..100) so the bars don't jump when `total` rises.
  const springs = new Map<string, Spring<number>>();
  function widthOf(opt: OptionPublic): number {
    const target = total > 0 ? (opt.count / total) * 100 : 0;
    let s = springs.get(opt.id);
    if (!s) {
      s = new Spring(target, { stiffness: 0.08, damping: 0.4 });
      springs.set(opt.id, s);
    } else {
      s.target = target;
    }
    return s.current;
  }

  let joinUrl = $derived(`${typeof window === 'undefined' ? '' : window.location.origin}/p/${data.poll.slug}`);

  let connected = $state(false);
  let es: EventSource | null = null;

  onMount(() => {
    es = new EventSource(`${data.apiBase}/api/polls/${data.poll.slug}/stream`);
    es.addEventListener('open', () => (connected = true));
    es.addEventListener('error', () => (connected = false));
    es.addEventListener('counts', (e) => {
      try {
        const payload = JSON.parse((e as MessageEvent).data) as CountsEvent;
        total = payload.total;
        counts = payload.counts.map((c) => ({
          id: c.option_id,
          label: c.label,
          position: c.position,
          count: c.count
        }));
      } catch (err) {
        console.warn('failed to parse counts event', err);
      }
    });

    return () => {
      es?.close();
    };
  });
</script>

<svelte:head><title>{data.poll.question}</title></svelte:head>

<header class="hero">
  <div>
    <p class="slug">/{data.poll.slug} <span class="status" class:open={isOpen}>{isOpen ? 'open' : 'closed'}</span></p>
    <h1>{data.poll.question}</h1>
    <p class="join">
      Join at <code>{joinUrl}</code>
      <span class="dot" class:connected aria-label={connected ? 'live' : 'connecting'}></span>
    </p>
  </div>
  <img class="qr" src={`${data.apiBase}/api/polls/${data.poll.slug}/qr`} alt={`QR code linking to ${joinUrl}`} width="180" height="180" />
</header>

<section class="bars" aria-label="Live results">
  {#each counts as opt (opt.id)}
    {@const w = widthOf(opt)}
    <div class="row">
      <div class="label">
        <span>{opt.label}</span>
        <span class="count">{opt.count}</span>
      </div>
      <div class="track">
        <div class="fill" style:width={`${w}%`}></div>
      </div>
    </div>
  {/each}
</section>

<footer class="foot">
  <p class="total">{total} total vote{total === 1 ? '' : 's'}</p>
  {#if isOpen}
    <form
      method="POST"
      action="?/close"
      use:enhance={() => {
        return async ({ update }) => {
          isOpen = false;
          await update();
        };
      }}
    >
      <button type="submit" class="danger">Close poll</button>
    </form>
  {/if}
</footer>

<style>
  .hero {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-5);
    align-items: start;
    margin-bottom: var(--space-5);
  }
  .slug {
    font-family: monospace;
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    margin-bottom: var(--space-2);
  }
  .status {
    text-transform: uppercase;
    font-weight: 700;
    font-size: var(--text-xs);
    padding: 2px 6px;
    border-radius: 999px;
    background: hsl(0 0% 50% / 0.15);
    color: hsl(0 0% 30%);
  }
  .status.open { background: hsl(142 70% 45% / 0.15); color: hsl(142 70% 25%); }
  h1 { font-size: var(--text-3xl); font-weight: 800; margin-bottom: var(--space-3); }
  .join { color: var(--color-fg-muted); display: flex; align-items: center; gap: var(--space-2); }
  .join code { background: var(--color-bg-elev); padding: 2px 6px; border-radius: var(--radius-sm); }
  .dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: hsl(0 0% 60%);
  }
  .dot.connected { background: hsl(142 70% 45%); box-shadow: 0 0 0 4px hsl(142 70% 45% / 0.2); }
  .qr {
    background: white;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2);
  }
  .bars { display: grid; gap: var(--space-4); }
  .row { display: grid; gap: var(--space-1); }
  .label {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .count { font-variant-numeric: tabular-nums; color: var(--color-fg-muted); }
  .track {
    height: 36px;
    background: var(--color-bg-elev);
    border-radius: var(--radius-md);
    overflow: hidden;
    border: 1px solid var(--color-border);
  }
  .fill {
    height: 100%;
    background: linear-gradient(90deg, var(--color-accent), hsl(220 90% 70%));
    border-radius: inherit;
  }
  .foot {
    margin-top: var(--space-5);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .total { color: var(--color-fg-muted); }
  .danger {
    padding: var(--space-2) var(--space-4);
    background: hsl(0 72% 51%);
    color: white;
    border-radius: var(--radius-md);
    font-weight: 600;
  }
  .danger:hover { background: hsl(0 72% 45%); }
</style>

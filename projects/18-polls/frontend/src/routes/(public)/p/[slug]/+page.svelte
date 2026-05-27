<script lang="ts">
  import { onMount } from 'svelte';
  import type { PageProps } from './$types';
  import type { CountsEvent, OptionPublic } from '$lib/types';
  import { pollsApi, ApiCallError } from '$lib/api';

  let { data }: PageProps = $props();

  let isOpen = $state(data.poll.is_open);
  let counts = $state<OptionPublic[]>(data.poll.options.map((o) => ({ ...o })));
  let total = $state(data.poll.total_votes);
  let myVote = $state<string | null>(null);
  let submitting = $state(false);
  let error: string | null = $state(null);

  let es: EventSource | null = null;

  onMount(() => {
    es = new EventSource(`${data.apiBase}/api/polls/${data.poll.slug}/stream`);
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
      } catch {
        /* ignore */
      }
    });
    return () => es?.close();
  });

  async function vote(optionId: string) {
    if (submitting || myVote || !isOpen) return;
    submitting = true;
    error = null;
    try {
      await pollsApi.vote(fetch, data.poll.slug, optionId);
      myVote = optionId;
    } catch (e) {
      if (e instanceof ApiCallError) {
        error = e.message;
        if (e.status === 409) myVote = optionId; // already voted, treat as locked
      } else {
        error = 'Failed to record vote';
      }
    } finally {
      submitting = false;
    }
  }
</script>

<svelte:head><title>{data.poll.question}</title></svelte:head>

<h1>{data.poll.question}</h1>

{#if !isOpen}
  <p class="closed" role="status">This poll is closed.</p>
{/if}

<ul class="opts" aria-busy={submitting}>
  {#each counts as opt (opt.id)}
    <li>
      <button
        type="button"
        class="opt"
        class:voted={myVote === opt.id}
        disabled={!isOpen || submitting || myVote !== null}
        onclick={() => vote(opt.id)}
      >
        <span class="lbl">{opt.label}</span>
        {#if myVote}
          <span class="cnt">{opt.count}</span>
        {/if}
      </button>
    </li>
  {/each}
</ul>

{#if myVote}
  <p class="thanks" role="status">Thanks — your vote is recorded. Total: {total}.</p>
{/if}
{#if error}
  <p class="err" role="alert">{error}</p>
{/if}

<style>
  h1 { font-size: var(--text-2xl); font-weight: 800; margin-bottom: var(--space-5); }
  .opts { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .opt {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4) var(--space-5);
    background: var(--color-bg-elev);
    border: 2px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-lg);
    text-align: left;
    cursor: pointer;
    color: inherit;
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .opt:hover:not(:disabled) { border-color: var(--color-accent); }
  .opt:disabled { cursor: not-allowed; }
  .opt.voted { border-color: var(--color-accent); background: hsl(220 90% 60% / 0.1); }
  .cnt { font-variant-numeric: tabular-nums; color: var(--color-fg-muted); }
  .closed {
    padding: var(--space-3) var(--space-4);
    background: hsl(0 0% 50% / 0.15);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-4);
  }
  .thanks {
    margin-top: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: hsl(142 70% 45% / 0.15);
    color: hsl(142 70% 25%);
    border-radius: var(--radius-md);
  }
  .err {
    margin-top: var(--space-4);
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
  }
</style>

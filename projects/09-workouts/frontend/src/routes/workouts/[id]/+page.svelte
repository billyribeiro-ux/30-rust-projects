<script lang="ts">
  import { enhance } from '$app/forms';
  import { ArrowLeft, Trash, Trophy } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PrBadge from '$lib/components/PrBadge.svelte';
  import { formatWeight, formatVolume } from '$lib/weight';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const workout = $derived(data.workout);
  const totalVolume = $derived(
    workout.sets.reduce((acc, s) => acc + s.volume_minor, 0)
  );
  const prCount = $derived(workout.sets.filter((s) => s.is_pr).length);

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }
</script>

<svelte:head>
  <title>{workout.name || 'Workout'} — Workouts</title>
</svelte:head>

<main>
  <p class="back"><a href="/"><Icon icon={ArrowLeft} size={14} /> Back to dashboard</a></p>

  <header>
    <h1>{workout.name || 'Workout'}</h1>
    <p class="lead">{formatDate(workout.performed_at)}</p>
    <dl class="meta">
      <div>
        <dt>Sets</dt>
        <dd>{workout.sets.length}</dd>
      </div>
      <div>
        <dt>Volume</dt>
        <dd>{formatVolume(totalVolume)}</dd>
      </div>
      <div>
        <dt>PRs</dt>
        <dd>
          <Icon icon={Trophy} size={14} weight="duotone" />
          {prCount}
        </dd>
      </div>
    </dl>
  </header>

  <section class="panel" aria-label="Sets">
    {#if workout.sets.length === 0}
      <p class="empty">This workout has no sets.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th scope="col">#</th>
            <th scope="col">Exercise</th>
            <th scope="col">Weight</th>
            <th scope="col">Reps</th>
            <th scope="col">RIR</th>
            <th scope="col">Volume</th>
            <th scope="col"><span class="sr-only">PR</span></th>
          </tr>
        </thead>
        <tbody>
          {#each workout.sets as set, i (set.id)}
            <tr class:pr-row={set.is_pr}>
              <td>{i + 1}</td>
              <td>{set.exercise_name}</td>
              <td>{formatWeight(set.weight_minor)}</td>
              <td>{set.reps}</td>
              <td>{set.rir}</td>
              <td>{formatVolume(set.volume_minor)}</td>
              <td data-testid={set.is_pr ? `pr-${set.id}` : undefined}>
                {#if set.is_pr}
                  <PrBadge />
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <form method="POST" action="?/remove" use:enhance={() => async ({ update }) => update()}>
    <button type="submit" class="danger" aria-label="Delete workout">
      <Icon icon={Trash} size={14} />
      <span>Delete workout</span>
    </button>
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }
  .back a {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  header { display: grid; gap: var(--space-2); }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .meta {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-bg-elev);
    border-radius: var(--radius-md);
  }
  .meta dt {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .meta dd {
    font-size: var(--text-lg);
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-variant-numeric: tabular-nums;
  }

  .panel {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow-x: auto;
  }
  table { width: 100%; border-collapse: collapse; }
  th, td {
    padding: var(--space-2) var(--space-3);
    text-align: left;
    border-bottom: 1px solid var(--color-border);
    font-variant-numeric: tabular-nums;
  }
  th {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--color-bg-sunken);
  }
  tbody tr:last-child td { border-bottom: none; }
  tbody tr.pr-row { background: hsl(38 92% 45% / 0.05); }

  .danger {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    color: var(--color-danger);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  .danger:hover {
    border-color: var(--color-danger);
    background: hsl(0 72% 51% / 0.05);
  }

  .empty {
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
  }

  .sr-only {
    position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0;
  }
</style>

<script lang="ts">
  import { Barbell, Plus, ListBullets, Trophy, Download } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { formatVolume } from '$lib/weight';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // The workouts list comes back with set counts and per-workout volumes
  // pre-aggregated by the backend. The dashboard's headline widgets are
  // a CHAIN of $derived values:
  //   workouts -> totalSets -> averageSetsPerWorkout
  //   workouts -> totalVolume -> volumeThisWeek
  //   workouts -> recentWorkouts (sliced for display)
  // Each `$derived` is a leaf node in a dependency graph. Touching `data`
  // (e.g., after invalidation) recomputes them lazily in topological order.
  const workouts = $derived(data.workouts);

  const totalSets = $derived(workouts.reduce((acc, w) => acc + w.set_count, 0));
  const totalVolume = $derived(workouts.reduce((acc, w) => acc + w.total_volume_minor, 0));

  // Local "last 7 days" derived from performed_at — independent of the
  // server's own count, demonstrating that the backend's stats payload
  // and the client's derived view agree.
  const oneWeekAgo = $derived(Date.now() - 7 * 24 * 60 * 60 * 1000);
  const last7Workouts = $derived(
    workouts.filter((w) => new Date(w.performed_at).getTime() >= oneWeekAgo)
  );
  const volumeThisWeek = $derived(
    last7Workouts.reduce((acc, w) => acc + w.total_volume_minor, 0)
  );
  const averageSetsPerWorkout = $derived(
    workouts.length === 0 ? 0 : Math.round(totalSets / workouts.length)
  );
  const recentWorkouts = $derived(workouts.slice(0, 5));

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={Barbell} size={28} weight="duotone" />
      <h1>Workouts</h1>
    </div>
    <p class="lead">Track sets. Spot PRs. Confirm you're getting stronger.</p>
  </header>

  <section class="stats" aria-label="Stats">
    <div>
      <dt>Workouts (7d)</dt>
      <dd data-testid="stat-last7">{data.stats.workouts_last_7_days}</dd>
    </div>
    <div>
      <dt>Total volume</dt>
      <dd data-testid="stat-volume">{formatVolume(totalVolume)}</dd>
    </div>
    <div>
      <dt>Total PRs</dt>
      <dd data-testid="stat-prs">{data.stats.pr_count_total}</dd>
    </div>
    <div>
      <dt>Avg sets / workout</dt>
      <dd data-testid="stat-avg">{averageSetsPerWorkout}</dd>
    </div>
  </section>

  <section class="panel" aria-label="This week">
    <h2>This week</h2>
    <p class="muted">
      {last7Workouts.length} workouts · {formatVolume(volumeThisWeek)} moved
    </p>
  </section>

  <section class="panel" aria-label="Recent workouts">
    <header class="panel-header">
      <h2>Recent workouts</h2>
      <div class="actions">
        <a class="primary" href="/workouts/new">
          <Icon icon={Plus} size={16} weight="bold" />
          <span>New workout</span>
        </a>
        <a class="secondary" href="/exercises">
          <Icon icon={ListBullets} size={16} />
          <span>Exercises</span>
        </a>
        <a class="secondary" href="http://localhost:3008/api/export.csv" data-testid="export-link">
          <Icon icon={Download} size={16} />
          <span>Export CSV</span>
        </a>
      </div>
    </header>

    {#if recentWorkouts.length === 0}
      <p class="empty">No workouts yet. Log your first to start tracking PRs.</p>
    {:else}
      <ul class="list">
        {#each recentWorkouts as w (w.id)}
          <li>
            <a href="/workouts/{w.id}">
              <div class="info">
                <p class="title">{w.name || 'Workout'}</p>
                <p class="muted">{formatDate(w.performed_at)}</p>
              </div>
              <div class="metrics">
                <p>{w.set_count} sets</p>
                <p class="muted">{formatVolume(w.total_volume_minor)}</p>
              </div>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <p class="footer-note">
    <Icon icon={Trophy} size={14} /> A set is a <strong>PR</strong> when its volume
    (weight × reps) strictly exceeds every prior set of that exercise.
  </p>
</main>

<style>
  main {
    max-width: var(--container-lg);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }
  header { display: grid; gap: var(--space-2); }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
  }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }

  .stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .stats > div { display: grid; gap: 2px; }
  .stats dt {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stats dd {
    font-size: var(--text-xl);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-4);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  h2 { font-size: var(--text-lg); font-weight: 700; }

  .actions { display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .primary,
  .secondary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--text-sm);
  }
  .primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }
  .primary:hover { background: var(--color-accent-hover); }
  .secondary {
    background: var(--color-bg);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
  }
  .secondary:hover { border-color: var(--color-border-strong); }

  .list { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .list a {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    color: inherit;
  }
  .list a:hover { border-color: var(--color-border-strong); }
  .title { font-weight: 600; }
  .muted { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .metrics { text-align: right; }
  .metrics p { font-variant-numeric: tabular-nums; }

  .empty {
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }

  .footer-note {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  @media (min-width: 768px) {
    main { padding: var(--space-12) var(--space-6); }
    .stats { grid-template-columns: repeat(4, 1fr); }
  }
</style>

<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { enhance } from '$app/forms';
  import { streamUrl } from '$lib/api';
  import type { JobEvent } from '$lib/types';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let liveEvents = $state<JobEvent[]>([]);

  // Live SSE feed — bumps `liveEvents` for the dashboard's "recent
  // activity" pane and triggers `invalidateAll()` to refresh counts.
  $effect(() => {
    if (typeof EventSource === 'undefined') return;
    const es = new EventSource(streamUrl(), { withCredentials: true });
    es.addEventListener('job', (ev) => {
      try {
        const data = JSON.parse((ev as MessageEvent).data) as JobEvent;
        liveEvents = [data, ...liveEvents].slice(0, 50);
        // Refresh the queues table + the visible jobs list.
        invalidateAll();
      } catch {
        /* ignore malformed */
      }
    });
    es.addEventListener('lag', () => invalidateAll());
    return () => es.close();
  });
</script>

<svelte:head><title>Jobs dashboard</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/jobs">Jobs</a>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <section class="queues" aria-label="Queues">
    <h2>Queues</h2>
    {#if data.queues.length === 0}
      <p class="empty">No jobs enqueued yet.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Queue</th><th>Pending</th><th>Running</th><th>Succeeded</th><th>Failed</th><th>Dead</th>
          </tr>
        </thead>
        <tbody>
          {#each data.queues as q (q.queue)}
            <tr>
              <td>{q.queue}</td>
              <td>{q.pending}</td>
              <td>{q.running}</td>
              <td>{q.succeeded}</td>
              <td>{q.failed}</td>
              <td>{q.dead}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <section class="enqueue" aria-label="Enqueue a job">
    <h2>Enqueue</h2>
    <form
      method="POST"
      action="?/enqueue"
      use:enhance={() => async ({ update }) => { await update(); invalidateAll(); }}
    >
      <label class="field">
        <span>Kind</span>
        <select name="kind">
          <option value="send_email">send_email</option>
          <option value="resize_image">resize_image</option>
          <option value="flaky_job">flaky_job</option>
        </select>
      </label>
      <label class="field full">
        <span>Payload (JSON)</span>
        <textarea name="payload" rows="2">{`{}`}</textarea>
      </label>
      <button type="submit" class="primary">Enqueue</button>
      {#if form && 'error' in form && form.error}
        <p class="error" role="alert">{form.error as string}</p>
      {/if}
    </form>
  </section>

  <section class="jobs" aria-label="Jobs">
    <header>
      <h2>Jobs</h2>
      <div class="filters">
        <a class:active={data.status === 'pending'} href={`/jobs?queue=${data.queue}&status=pending`}>Pending</a>
        <a class:active={data.status === 'running'} href={`/jobs?queue=${data.queue}&status=running`}>Running</a>
        <a class:active={data.status === 'succeeded'} href={`/jobs?queue=${data.queue}&status=succeeded`}>Succeeded</a>
        <a class:active={data.status === 'dead'} href={`/jobs?queue=${data.queue}&status=dead`}>Dead</a>
      </div>
    </header>
    {#if data.jobs.length === 0}
      <p class="empty">No jobs in <code>{data.status}</code>.</p>
    {:else}
      <ul>
        {#each data.jobs as job (job.id)}
          <li class={`status-${job.status}`}>
            <span class="kind">{job.kind}</span>
            <span class="meta">attempt {job.attempts}/{job.max_attempts}</span>
            {#if job.last_error}
              <span class="err" title={job.last_error}>{job.last_error}</span>
            {/if}
            {#if job.status === 'dead' || job.status === 'failed'}
              <form method="POST" action="?/retry" use:enhance={() => async ({ update }) => { await update(); invalidateAll(); }}>
                <input type="hidden" name="id" value={job.id} />
                <button type="submit" class="ghost">Retry</button>
              </form>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="live" aria-label="Live activity">
    <h2>Live activity</h2>
    {#if liveEvents.length === 0}
      <p class="empty">Waiting for events… (try enqueuing one above)</p>
    {:else}
      <ul class="events">
        {#each liveEvents as ev (`${ev.job_id}-${ev.attempts}-${ev.status}`)}
          <li>
            <code class={`status-${ev.status}`}>{ev.status}</code>
            <span>{ev.kind}</span>
            <small>attempt {ev.attempts}</small>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  .topnav { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .brand { font-weight: 800; color: var(--color-accent); flex: 1; }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; cursor: pointer; }
  .who { color: var(--color-fg-muted); font-size: var(--text-sm); }
  main { max-width: 1080px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-6); }
  section { background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: var(--space-4); }
  h2 { font-size: var(--text-lg); font-weight: 700; margin-bottom: var(--space-3); }
  table { width: 100%; border-collapse: collapse; font-size: var(--text-sm); }
  th, td { padding: var(--space-2) var(--space-3); text-align: left; border-bottom: 1px solid var(--color-border); }
  th { color: var(--color-fg-muted); font-weight: 600; }
  .empty { color: var(--color-fg-muted); }
  .enqueue form { display: grid; grid-template-columns: 1fr 1fr auto; gap: var(--space-3); align-items: end; }
  .field { display: grid; gap: var(--space-1); }
  .field.full { grid-column: 1 / -1; }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field select, .field textarea { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); font-family: var(--font-mono, monospace); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .error { color: var(--color-danger); font-size: var(--text-sm); grid-column: 1 / -1; }
  .jobs header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); }
  .filters { display: flex; gap: var(--space-2); }
  .filters a { padding: var(--space-1) var(--space-3); color: var(--color-fg-muted); text-decoration: none; border-radius: var(--radius-pill); font-size: var(--text-sm); }
  .filters a.active { color: var(--color-accent-fg); background: var(--color-accent); }
  .jobs ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .jobs li { display: grid; grid-template-columns: 1fr auto 2fr auto; gap: var(--space-3); padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); align-items: center; }
  .kind { font-family: var(--font-mono, monospace); font-weight: 500; }
  .meta { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .err { color: var(--color-danger); font-size: var(--text-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .events { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-1); max-height: 320px; overflow-y: auto; }
  .events li { display: grid; grid-template-columns: auto 1fr auto; gap: var(--space-2); align-items: center; padding: var(--space-1) var(--space-2); font-size: var(--text-sm); }
  .events code { padding: 0 var(--space-2); border-radius: var(--radius-sm); font-size: var(--text-xs); text-transform: uppercase; }
  .status-running { background: hsl(45 100% 50% / 0.18); color: hsl(35 100% 30%); }
  .status-succeeded { background: hsl(140 60% 50% / 0.18); color: hsl(140 60% 25%); }
  .status-pending { background: hsl(220 30% 60% / 0.18); color: var(--color-fg-muted); }
  .status-failed, .status-dead { background: hsl(0 72% 51% / 0.12); color: var(--color-danger); }
</style>

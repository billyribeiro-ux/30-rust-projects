<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import { STATUS_LABEL, type AppStatus } from '$lib/types';

  let { data }: PageProps = $props();
  const app = $derived(data.app);
  const events = $derived(data.events);
  const nextSteps = $derived(data.nextSteps);

  const PIPELINE: AppStatus[] = [
    'wishlist',
    'applied',
    'screening',
    'interview',
    'offer',
    'accepted'
  ];
  const isPast = (s: AppStatus) => PIPELINE.indexOf(s) < PIPELINE.indexOf(app.status);
  const isCurrent = (s: AppStatus) => s === app.status;
</script>

<svelte:head><title>{app.company} — {app.role}</title></svelte:head>

<main>
  <a class="back" href="/applications">← Back to applications</a>

  <article class="hero">
    <h1>{app.company}</h1>
    <p class="role">{app.role}</p>
    {#if app.location}<p class="location">{app.location}</p>{/if}
    {#if app.job_url}
      <p class="link"><a href={app.job_url} target="_blank" rel="noopener noreferrer">Job posting ↗</a></p>
    {/if}
  </article>

  <!-- STATUS TIMELINE: visual representation of pipeline progress. The
       current status is highlighted; past statuses are completed; future
       are dim. Click any to advance the application (POSTs updateStatus). -->
  <section class="panel" aria-label="Status pipeline">
    <h2>Pipeline</h2>
    <ol class="timeline">
      {#each PIPELINE as s, i (s)}
        <li
          class="step"
          class:past={isPast(s)}
          class:current={isCurrent(s)}
          aria-current={isCurrent(s) ? 'step' : undefined}
        >
          <form method="POST" action="?/updateStatus" use:enhance>
            <input type="hidden" name="status" value={s} />
            <button type="submit" class="step-button">
              <span class="dot" aria-hidden="true">{i + 1}</span>
              <span class="label">{STATUS_LABEL[s]}</span>
            </button>
          </form>
        </li>
      {/each}
    </ol>
    {#if app.status === 'rejected' || app.status === 'withdrawn'}
      <p class="terminal">This application is <strong>{STATUS_LABEL[app.status]}</strong>.</p>
    {:else}
      <details class="terminal-actions">
        <summary>Mark as rejected or withdrawn…</summary>
        <div class="actions">
          {#each ['rejected', 'withdrawn'] as s (s)}
            <form method="POST" action="?/updateStatus" use:enhance>
              <input type="hidden" name="status" value={s} />
              <button type="submit" class="ghost">{STATUS_LABEL[s as AppStatus]}</button>
            </form>
          {/each}
        </div>
      </details>
    {/if}
  </section>

  <section class="panel" aria-label="Next steps">
    <h2>Next steps</h2>
    <form method="POST" action="?/addStep" use:enhance class="add-step">
      <input type="text" name="body" placeholder="What's the next thing to do?" maxlength="200" required />
      <input type="datetime-local" name="due_at" required />
      <button type="submit" class="primary">Add step</button>
    </form>
    {#if nextSteps.length === 0}
      <p class="empty">No upcoming steps.</p>
    {:else}
      <ul class="steps">
        {#each nextSteps as ns (ns.id)}
          <li class:complete={!!ns.completed_at}>
            <form method="POST" action="?/completeStep" use:enhance>
              <input type="hidden" name="step_id" value={ns.id} />
              <button type="submit" class="check" aria-label={`Mark "${ns.body}" complete`} disabled={!!ns.completed_at}>
                {#if ns.completed_at}✓{:else}○{/if}
              </button>
            </form>
            <div class="step-meta">
              <p>{ns.body}</p>
              <time datetime={ns.due_at}>{new Date(ns.due_at).toLocaleString()}</time>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel" aria-label="Timeline">
    <h2>Timeline</h2>
    <form method="POST" action="?/addNote" use:enhance class="add-note">
      <textarea name="body" placeholder="Add a note (e.g., 'spoke with recruiter')" maxlength="2000" rows="2" required></textarea>
      <button type="submit" class="primary">Add note</button>
    </form>
    {#if events.length === 0}
      <p class="empty">No events yet.</p>
    {:else}
      <ol class="events">
        {#each events as e (e.id)}
          <li>
            <span class="event-kind event-{e.kind}">{e.kind === 'status_change' ? '→' : e.kind === 'note' ? '📝' : '·'}</span>
            <div>
              <p class="event-body">
                {#if e.kind === 'status_change' && e.new_status}
                  Status changed to <strong>{STATUS_LABEL[e.new_status]}</strong>
                {:else}
                  {e.body}
                {/if}
              </p>
              <time datetime={e.occurred_at}>{new Date(e.occurred_at).toLocaleString()}</time>
            </div>
          </li>
        {/each}
      </ol>
    {/if}
  </section>

  <section class="panel danger-panel" aria-label="Danger">
    <form
      method="POST"
      action="?/remove"
      onsubmit={(e) => { if (!confirm(`Delete ${app.company} — ${app.role}? This deletes all events + next steps.`)) e.preventDefault(); }}
    >
      <button type="submit" class="danger">Delete this application</button>
    </form>
  </section>
</main>

<style>
  main { max-width: var(--container-md); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  .back { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .hero { padding: var(--space-6); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: grid; gap: var(--space-2); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .role { font-size: var(--text-lg); color: var(--color-fg); }
  .location { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .link a { font-size: var(--text-sm); }
  .panel { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: grid; gap: var(--space-3); }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  .timeline { list-style: none; padding: 0; margin: 0; display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .step { flex: 1 1 110px; }
  .step-button {
    display: grid; place-items: center; gap: var(--space-1);
    width: 100%; padding: var(--space-3) var(--space-2);
    border: 2px solid var(--color-border);
    background: var(--color-bg); border-radius: var(--radius-md);
    color: var(--color-fg-muted); font-size: var(--text-xs);
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .step.past .step-button { border-color: hsl(142 71% 35% / 0.5); color: hsl(142 71% 28%); }
  .step.current .step-button { border-color: var(--color-accent); color: var(--color-accent); font-weight: 700; }
  .step-button:hover { border-color: var(--color-border-strong); }
  .dot { display: inline-grid; place-items: center; width: 22px; height: 22px; border-radius: 999px; background: var(--color-bg-sunken); font-weight: 700; font-size: var(--text-xs); }
  .step.past .dot { background: hsl(142 71% 35% / 0.15); color: hsl(142 71% 28%); }
  .step.current .dot { background: var(--color-accent); color: var(--color-accent-fg); }
  .terminal { padding: var(--space-3); background: var(--color-bg-sunken); border-radius: var(--radius-sm); color: var(--color-fg-muted); text-align: center; }
  .terminal-actions { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .terminal-actions summary { cursor: pointer; }
  .terminal-actions .actions { display: flex; gap: var(--space-2); margin-top: var(--space-2); }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); background: var(--color-bg); }
  .add-step { display: grid; gap: var(--space-2); }
  .add-step input { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
  .add-note { display: grid; gap: var(--space-2); }
  .add-note textarea { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-family: var(--font-sans); resize: vertical; }
  .primary {
    justify-self: start;
    padding: var(--space-2) var(--space-4);
    background: var(--color-accent); color: var(--color-accent-fg);
    border-radius: var(--radius-md); font-weight: 600;
  }
  .steps { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .steps li { display: flex; gap: var(--space-2); align-items: flex-start; padding: var(--space-2) var(--space-3); background: var(--color-bg); border-radius: var(--radius-sm); }
  .steps li.complete .step-meta { color: var(--color-fg-muted); text-decoration: line-through; }
  .check { width: 24px; height: 24px; border-radius: 999px; background: var(--color-bg-sunken); color: var(--color-fg-muted); font-size: var(--text-sm); }
  .check[disabled] { color: hsl(142 71% 28%); background: hsl(142 71% 35% / 0.12); cursor: default; }
  .step-meta { flex: 1; }
  .step-meta time { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .events { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .events li { display: flex; gap: var(--space-3); }
  .event-kind { display: inline-grid; place-items: center; width: 24px; height: 24px; border-radius: 999px; background: var(--color-bg-sunken); }
  .event-body { font-size: var(--text-sm); }
  .events time { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .empty { color: var(--color-fg-muted); padding: var(--space-3); background: var(--color-bg-sunken); border-radius: var(--radius-sm); text-align: center; }
  .danger-panel { border-color: hsl(0 72% 51% / 0.4); }
  .danger { padding: var(--space-2) var(--space-4); color: hsl(0 72% 42%); border: 1px solid hsl(0 72% 51% / 0.4); border-radius: var(--radius-md); font-weight: 600; background: var(--color-bg); }
</style>

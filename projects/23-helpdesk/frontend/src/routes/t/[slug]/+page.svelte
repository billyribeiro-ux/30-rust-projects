<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>Tickets — {data.slug}</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/tenants">Help Desk</a>
  <span class="meta">/{data.slug}</span>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <section class="head">
    <h1>Tickets</h1>
    <div class="filters">
      {#each ['open', 'pending', 'resolved', 'closed'] as st (st)}
        <a class:active={data.status === st} href={`/t/${data.slug}?status=${st}`}>{st}</a>
      {/each}
    </div>
  </section>

  <form
    method="POST"
    action="?/create"
    class="create"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => { submitting = false; await update(); };
    }}
  >
    <label class="field">
      <span>Subject</span>
      <input name="subject" required maxlength="200" />
    </label>
    <label class="field">
      <span>Priority</span>
      <select name="priority">
        <option>low</option><option selected>normal</option><option>high</option><option>urgent</option>
      </select>
    </label>
    <label class="field full">
      <span>Body</span>
      <textarea name="body" rows="2" required></textarea>
    </label>
    <button type="submit" class="primary" disabled={submitting}>{submitting ? '…' : 'Create ticket'}</button>
    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error as string}</p>
    {/if}
  </form>

  {#if data.tickets.length === 0}
    <p class="empty">No tickets in <code>{data.status}</code>.</p>
  {:else}
    <ul>
      {#each data.tickets as t (t.id)}
        <li>
          <a class="card" href={`/t/${data.slug}/${t.id}`}>
            <span class="subject">{t.subject}</span>
            <span class={`priority p-${t.priority}`}>{t.priority}</span>
            <time datetime={t.created_at}>{new Date(t.created_at).toLocaleString()}</time>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .topnav { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .brand { font-weight: 800; color: var(--color-accent); }
  .meta { color: var(--color-fg-muted); font-size: var(--text-sm); flex: 1; }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; }
  .who { color: var(--color-fg-muted); font-size: var(--text-sm); }
  main { max-width: 960px; margin: 0 auto; padding: var(--space-5) var(--space-4); display: grid; gap: var(--space-4); }
  .head { display: flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; }
  h1 { font-size: var(--text-2xl); font-weight: 800; flex: 1; }
  .filters { display: flex; gap: var(--space-1); }
  .filters a { padding: var(--space-1) var(--space-3); border-radius: var(--radius-pill); color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  .filters a.active { background: var(--color-accent); color: var(--color-accent-fg); }
  .create { display: grid; grid-template-columns: 2fr auto auto; gap: var(--space-3); align-items: end; padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .field { display: grid; gap: var(--space-1); }
  .field.full { grid-column: 1 / -1; }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input, .field select, .field textarea { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .error { grid-column: 1 / -1; color: var(--color-danger); font-size: var(--text-sm); }
  .empty { color: var(--color-fg-muted); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .card { display: grid; grid-template-columns: 1fr auto auto; gap: var(--space-3); padding: var(--space-3); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; align-items: center; }
  .subject { font-weight: 500; }
  .priority { padding: 0 var(--space-2); font-size: var(--text-xs); border-radius: var(--radius-pill); text-transform: uppercase; }
  .p-low { background: hsl(220 30% 60% / 0.18); color: var(--color-fg-muted); }
  .p-normal { background: hsl(220 30% 60% / 0.18); color: var(--color-fg-muted); }
  .p-high { background: hsl(45 100% 50% / 0.2); color: hsl(35 100% 30%); }
  .p-urgent { background: hsl(0 72% 51% / 0.18); color: var(--color-danger); }
  time { color: var(--color-fg-muted); font-size: var(--text-xs); }
  @media (max-width: 640px) { .create { grid-template-columns: 1fr; } }
</style>

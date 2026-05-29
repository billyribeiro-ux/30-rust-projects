<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>Interviews</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/">Interviews</a>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <h1>Your interviews</h1>

  <form
    method="POST"
    action="?/create"
    class="create"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => { submitting = false; await update(); };
    }}
  >
    <label class="field"><span>Candidate email</span>
      <input type="email" name="candidate_email" required />
    </label>
    <label class="field"><span>Language</span>
      <select name="language">
        <option>rust</option><option>python</option><option>typescript</option>
      </select>
    </label>
    <button type="submit" class="primary" disabled={submitting}>{submitting ? '…' : 'Schedule'}</button>
    {#if form && 'error' in form && form.error}<p class="error" role="alert">{form.error as string}</p>{/if}
  </form>

  {#if data.interviews.length === 0}
    <p class="empty">No interviews yet.</p>
  {:else}
    <ul>
      {#each data.interviews as iv (iv.id)}
        <li>
          <a class="card" href={`/i/${iv.id}`}>
            <span class="email">{iv.candidate_email}</span>
            <span class="lang">{iv.language}</span>
            <span class={`status status-${iv.status}`}>{iv.status}</span>
            <time datetime={iv.created_at}>{new Date(iv.created_at).toLocaleString()}</time>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .topnav { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .brand { font-weight: 800; color: var(--color-accent); flex: 1; }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; }
  .who { color: var(--color-fg-muted); font-size: var(--text-sm); }
  main { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .create { display: grid; grid-template-columns: 2fr 1fr auto; gap: var(--space-3); align-items: end; padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input, .field select { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .error { grid-column: 1 / -1; color: var(--color-danger); font-size: var(--text-sm); }
  .empty { color: var(--color-fg-muted); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .card { display: grid; grid-template-columns: 1fr auto auto auto; gap: var(--space-3); padding: var(--space-3); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; align-items: center; }
  .email { font-weight: 500; }
  .lang, time { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .status { padding: 0 var(--space-2); border-radius: var(--radius-pill); font-size: var(--text-xs); text-transform: uppercase; }
  .status-scheduled { background: hsl(220 30% 60% / 0.18); color: var(--color-fg-muted); }
  .status-live { background: hsl(140 60% 50% / 0.18); color: hsl(140 60% 25%); }
  .status-ended { background: hsl(0 72% 51% / 0.12); color: var(--color-danger); }
  @media (max-width: 640px) { .create { grid-template-columns: 1fr; } }
</style>

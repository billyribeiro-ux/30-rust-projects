<script lang="ts">
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);

  // Show the freshly-created secret exactly once.
  let createdSecret = $derived(
    form && 'created' in form
      ? (form.created as { id: string; secret: string; prefix: string })
      : null
  );
</script>

<svelte:head><title>API keys — AI Inference</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/keys">AI API</a>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <a class="ghost" href="/passkeys">Passkeys</a>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <h1>API keys</h1>

  <section aria-labelledby="usage-h" class="usage">
    <h2 id="usage-h">This month</h2>
    <div class="cards">
      <article>
        <h3>Calls</h3>
        <p class="value">{data.usage.calls_this_month.toLocaleString()}</p>
      </article>
      <article>
        <h3>Estimated bill</h3>
        <p class="value">${(data.usage.estimated_cents / 100).toFixed(2)}</p>
      </article>
      <article>
        <h3>Free tier</h3>
        <p class="value">{data.usage.free_tier_calls.toLocaleString()}</p>
      </article>
    </div>
  </section>

  <form
    method="POST"
    action="?/create"
    class="create"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => { submitting = false; await update(); invalidateAll(); };
    }}
  >
    <label class="field"><span>Name</span><input name="name" required maxlength="80" /></label>
    <label class="field"><span>Rate limit (RPM)</span><input type="number" name="rpm_limit" value="60" min="1" max="10000" /></label>
    <button type="submit" class="primary" disabled={submitting}>{submitting ? '…' : 'Create key'}</button>
    {#if form && 'error' in form && form.error}<p class="error" role="alert">{form.error as string}</p>{/if}
  </form>

  {#if createdSecret}
    <aside class="reveal" role="status">
      <p><strong>One-time secret:</strong></p>
      <code class="secret">{createdSecret.secret}</code>
      <p class="note">Copy this now — we won't show it again.</p>
    </aside>
  {/if}

  {#if data.keys.length === 0}
    <p class="empty">No keys yet. Create one above.</p>
  {:else}
    <table>
      <thead>
        <tr><th>Name</th><th>Prefix</th><th>RPM</th><th>Last used</th><th></th></tr>
      </thead>
      <tbody>
        {#each data.keys as k (k.id)}
          <tr class:revoked={k.revoked}>
            <td>{k.name}</td>
            <td><code>{k.prefix}…</code></td>
            <td>{k.rpm_limit}</td>
            <td>{k.last_used_at ? new Date(k.last_used_at).toLocaleString() : '—'}</td>
            <td>
              {#if !k.revoked}
                <form method="POST" action="?/revoke" use:enhance={() => async ({ update }) => { await update(); invalidateAll(); }}>
                  <input type="hidden" name="id" value={k.id} />
                  <button type="submit" class="ghost">Revoke</button>
                </form>
              {:else}
                <span class="badge">revoked</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

<style>
  .topnav { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .brand { font-weight: 800; color: var(--color-accent); flex: 1; }
  .ghost { padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; cursor: pointer; }
  .who { color: var(--color-fg-muted); font-size: var(--text-sm); }
  main { max-width: 960px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  h2 { font-size: var(--text-sm); color: var(--color-fg-muted); text-transform: uppercase; letter-spacing: 0.04em; font-weight: 700; }
  .usage .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: var(--space-3); margin-top: var(--space-2); }
  .usage article { background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: var(--space-3); }
  .usage h3 { font-size: var(--text-xs); color: var(--color-fg-muted); text-transform: uppercase; letter-spacing: 0.04em; }
  .usage .value { font-size: var(--text-xl); font-weight: 800; margin-top: var(--space-1); }
  .create { display: grid; grid-template-columns: 2fr 1fr auto; gap: var(--space-3); align-items: end; padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .primary:disabled { opacity: 0.5; }
  .error { grid-column: 1 / -1; color: var(--color-danger); font-size: var(--text-sm); }
  .reveal { padding: var(--space-4); background: hsl(140 60% 50% / 0.08); border: 1px solid hsl(140 60% 50% / 0.4); border-radius: var(--radius-md); }
  .secret { font-family: var(--font-mono, monospace); background: var(--color-bg); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); display: inline-block; margin-top: var(--space-2); word-break: break-all; }
  .note { color: var(--color-fg-muted); font-size: var(--text-sm); margin-top: var(--space-2); }
  .empty { color: var(--color-fg-muted); }
  table { width: 100%; border-collapse: collapse; background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  th, td { padding: var(--space-2) var(--space-3); text-align: left; border-bottom: 1px solid var(--color-border); }
  th { color: var(--color-fg-muted); font-weight: 600; font-size: var(--text-sm); }
  .revoked { color: var(--color-fg-muted); }
  .badge { font-size: var(--text-xs); padding: 0 var(--space-2); border-radius: var(--radius-pill); background: var(--color-bg-sunken); color: var(--color-fg-muted); text-transform: uppercase; }
  code { font-family: var(--font-mono, monospace); }
  @media (max-width: 640px) { .create { grid-template-columns: 1fr; } }
</style>

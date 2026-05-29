<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let email = $state('');
  let plan = $state<'free' | 'pro'>('pro');
  let submitting = $state(false);
</script>

<svelte:head>
  <title>Newsletter — posts and Pro subscription</title>
</svelte:head>

<header class="hero">
  <h1>The Newsletter</h1>
  <p class="lead">Free posts on the homepage. Pro members read the gated ones.</p>

  <form
    method="POST"
    action="?/subscribe"
    class="subscribe"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => { submitting = false; await update(); };
    }}
  >
    <input
      type="email"
      name="email"
      placeholder="you@example.com"
      required
      bind:value={email}
      aria-label="Email"
    />
    <label class="plan">
      <input type="radio" name="plan" value="free" checked={plan === 'free'} onchange={() => (plan = 'free')} />
      Free
    </label>
    <label class="plan">
      <input type="radio" name="plan" value="pro" checked={plan === 'pro'} onchange={() => (plan = 'pro')} />
      Pro
    </label>
    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Working…' : 'Subscribe'}
    </button>
    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error as string}</p>
    {/if}
    {#if form && 'message' in form && form.message}
      <p class="ok">{form.message as string}</p>
    {/if}
  </form>
</header>

<section aria-labelledby="posts-h">
  <h2 id="posts-h">Latest posts</h2>
  {#if data.error}
    <p class="error" role="alert">{data.error}</p>
  {:else if data.posts.length === 0}
    <p class="empty">No posts yet.</p>
  {:else}
    <ul>
      {#each data.posts as p (p.id)}
        <li>
          <a class="card" href={`/p/${p.slug}`}>
            <span class="title">{p.title}</span>
            {#if p.is_gated}<span class="badge">Pro</span>{/if}
            <span class="summary">{p.summary}</span>
            {#if p.published_at}
              <time datetime={p.published_at}>{new Date(p.published_at).toLocaleDateString()}</time>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .hero { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin-bottom: var(--space-4); }
  .subscribe { display: grid; grid-template-columns: 2fr auto auto auto; gap: var(--space-2); align-items: center; }
  .subscribe input[type="email"] { padding: var(--space-3) var(--space-4); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .plan { display: inline-flex; align-items: center; gap: var(--space-1); font-size: var(--text-sm); color: var(--color-fg-muted); }
  .primary { padding: var(--space-3) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .error { grid-column: 1 / -1; color: var(--color-danger); font-size: var(--text-sm); }
  .ok { grid-column: 1 / -1; color: var(--color-success, var(--color-accent)); font-size: var(--text-sm); }
  section { max-width: 720px; margin: 0 auto; padding: 0 var(--space-4) var(--space-8); }
  h2 { font-size: var(--text-xl); font-weight: 700; margin-bottom: var(--space-3); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .card { display: grid; grid-template-columns: 1fr auto; column-gap: var(--space-2); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; }
  .title { font-weight: 600; }
  .badge { font-size: var(--text-xs); padding: 0 var(--space-2); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-pill); align-self: start; }
  .summary { grid-column: 1 / -1; color: var(--color-fg-muted); font-size: var(--text-sm); }
  time { grid-column: 1 / -1; color: var(--color-fg-muted); font-size: var(--text-xs); }
  .empty { color: var(--color-fg-muted); }

  @media (max-width: 640px) { .subscribe { grid-template-columns: 1fr; } }
</style>

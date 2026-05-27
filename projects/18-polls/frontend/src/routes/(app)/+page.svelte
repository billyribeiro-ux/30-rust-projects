<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<svelte:head><title>My polls</title></svelte:head>

<header class="hero">
  <h1>My polls</h1>
  <a class="primary" href="/polls/new">New poll</a>
</header>

{#if data.polls.length === 0}
  <p class="empty">
    No polls yet. <a href="/polls/new">Create one</a> — share the QR code with your audience
    and watch live bars animate as votes arrive.
  </p>
{:else}
  <ul class="poll-list">
    {#each data.polls as p (p.id)}
      <li>
        <a class="card" href={`/polls/${p.slug}`}>
          <div class="row1">
            <span class="status" class:open={p.is_open} class:closed={!p.is_open}>
              {p.is_open ? 'open' : 'closed'}
            </span>
            <span class="slug">/{p.slug}</span>
          </div>
          <h2>{p.question}</h2>
          <div class="meta">
            <span>{p.option_count} option{p.option_count === 1 ? '' : 's'}</span>
            <span>·</span>
            <span>{p.vote_count} vote{p.vote_count === 1 ? '' : 's'}</span>
          </div>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-5);
  }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .primary {
    padding: var(--space-2) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    text-decoration: none;
  }
  .primary:hover { background: var(--color-accent-hover); }
  .empty {
    padding: var(--space-5);
    color: var(--color-fg-muted);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-md);
    text-align: center;
  }
  .empty a { color: var(--color-accent); }
  .poll-list { display: grid; gap: var(--space-3); list-style: none; padding: 0; margin: 0; }
  .card {
    display: block;
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    color: inherit;
    text-decoration: none;
  }
  .card:hover { border-color: var(--color-border-strong); }
  .row1 {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
    margin-bottom: var(--space-2);
  }
  .status {
    text-transform: uppercase;
    font-weight: 700;
    font-size: var(--text-xs);
    padding: 2px 6px;
    border-radius: 999px;
  }
  .status.open { background: hsl(142 70% 45% / 0.15); color: hsl(142 70% 25%); }
  .status.closed { background: hsl(0 0% 50% / 0.15); color: hsl(0 0% 30%); }
  h2 { font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-2); }
  .meta { font-size: var(--text-sm); color: var(--color-fg-muted); display: flex; gap: var(--space-2); }
</style>

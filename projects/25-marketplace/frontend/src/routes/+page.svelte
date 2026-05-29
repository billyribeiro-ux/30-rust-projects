<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<svelte:head><title>Marketplace — courses</title></svelte:head>

<main>
  <header class="hero">
    <h1>Courses</h1>
    <p class="lead">Buy from independent instructors. The platform takes 10%; the rest goes directly to the instructor's Stripe account.</p>
    {#if !data.user}
      <p><a class="primary" href="/signup">Sign up</a></p>
    {:else}
      <p><a class="ghost" href="/instructor">Become an instructor</a></p>
    {/if}
  </header>

  {#if data.error}
    <p class="error" role="alert">{data.error}</p>
  {:else if data.courses.length === 0}
    <p class="empty">No published courses yet.</p>
  {:else}
    <ul>
      {#each data.courses as c (c.id)}
        <li>
          <a class="card" href={`/c/${c.slug}`}>
            <span class="title">{c.title}</span>
            <span class="instructor">by {c.instructor_name}</span>
            <span class="price">${(c.price_cents / 100).toFixed(2)} {c.currency.toUpperCase()}</span>
            {#if c.summary}<span class="summary">{c.summary}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main { max-width: 960px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  .hero h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin: var(--space-2) 0 var(--space-3); }
  .primary { display: inline-block; padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; text-decoration: none; }
  .ghost { display: inline-block; padding: var(--space-1) var(--space-3); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-sm); text-decoration: none; }
  .empty, .error { color: var(--color-fg-muted); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: var(--space-3); }
  .card { display: grid; gap: var(--space-1); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; }
  .title { font-weight: 600; }
  .instructor, .summary { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .price { color: var(--color-accent); font-weight: 700; }
</style>

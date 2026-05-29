<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<svelte:head>
  <title>SaaS Capstone — Project management for distributed teams</title>
</svelte:head>

<main>
  <section class="hero">
    <h1>Project management,<br />the boring correct way.</h1>
    <p class="lead">Multi-tenant. RLS-isolated. Stripe-billed. Audit-trail by Postgres trigger, not by hope.</p>
    <div class="cta">
      {#if data.user}
        <a class="primary" href="/p">Go to your workspace</a>
      {:else}
        <a class="primary" href="/signup">Start free</a>
        <a class="ghost" href="/login">Sign in</a>
      {/if}
      <a class="ghost" href="/pricing">Pricing</a>
    </div>
  </section>

  <section aria-labelledby="features-h">
    <h2 id="features-h">Engineered like the other 29 projects in the curriculum</h2>
    <ul>
      <li>
        <h3>Postgres RLS</h3>
        <p>Every tenant-scoped query runs inside a `SET LOCAL` transaction. App-level auth bug → no data leak.</p>
      </li>
      <li>
        <h3>Audit log via trigger</h3>
        <p>Every mutation on tasks and projects writes an `audit_log` row. The app cannot forget.</p>
      </li>
      <li>
        <h3>Stripe Subscriptions (groundwork)</h3>
        <p>`subscriptions` mirrors Stripe state per tenant. Webhook endpoint scaffolded — wire-up reuses project 21.</p>
      </li>
      <li>
        <h3>Built on the primitives</h3>
        <p>Auth (11), sessions (14), RLS (23), Stripe (16/21), background jobs (22), search (24), KPIs (27) — all the lessons.</p>
      </li>
    </ul>
  </section>
</main>

<style>
  main { max-width: 960px; margin: 0 auto; padding: var(--space-7) var(--space-4); display: grid; gap: var(--space-7); }
  .hero h1 { font-size: clamp(2.5rem, 6vw, 5rem); font-weight: 800; letter-spacing: -0.02em; line-height: 1.05; background: linear-gradient(135deg, var(--color-fg), hsl(220 50% 50%)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .lead { color: var(--color-fg-muted); font-size: var(--text-lg); margin-top: var(--space-3); max-width: 36em; }
  .cta { display: flex; gap: var(--space-3); margin-top: var(--space-5); flex-wrap: wrap; }
  .primary { padding: var(--space-3) var(--space-5); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; text-decoration: none; }
  .ghost { padding: var(--space-3) var(--space-4); color: var(--color-fg); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; }
  h2 { font-size: var(--text-2xl); font-weight: 700; margin-bottom: var(--space-4); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: var(--space-4); }
  li { padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  li h3 { font-size: var(--text-lg); font-weight: 700; margin-bottom: var(--space-2); }
  li p { color: var(--color-fg-muted); }
</style>

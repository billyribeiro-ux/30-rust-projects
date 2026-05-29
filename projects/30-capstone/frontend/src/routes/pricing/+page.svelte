<script lang="ts">
  let {} = $props();
</script>

<svelte:head><title>Pricing — Capstone</title></svelte:head>

<main>
  <h1>Pricing</h1>
  <ul class="plans">
    <li>
      <h2>Free</h2>
      <p class="price">$0</p>
      <ul>
        <li>1 workspace</li>
        <li>3 members</li>
        <li>Unlimited projects</li>
      </ul>
    </li>
    <li class="featured">
      <h2>Pro</h2>
      <p class="price">$12 <span>/ seat / month</span></p>
      <ul>
        <li>Unlimited workspaces</li>
        <li>Audit log access</li>
        <li>SAML SSO (project 26 pattern)</li>
        <li>Priority email support</li>
      </ul>
      <a class="primary" href="/signup">Start 14-day trial</a>
    </li>
  </ul>
  <p class="note">Stripe Subscriptions integration ships per project 21's pattern. Webhook endpoint is at <code>/api/stripe/webhook</code>.</p>
</main>

<style>
  main { max-width: 880px; margin: 0 auto; padding: var(--space-7) var(--space-4); display: grid; gap: var(--space-5); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .plans { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: var(--space-4); }
  .plans > li { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .featured { border-color: var(--color-accent); box-shadow: 0 0 0 2px hsl(220 70% 50% / 0.2); }
  .plans h2 { font-size: var(--text-xl); font-weight: 700; }
  .price { font-size: var(--text-3xl); font-weight: 800; margin: var(--space-2) 0; }
  .price span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .plans li ul { padding-left: var(--space-4); color: var(--color-fg-muted); }
  .primary { display: inline-block; margin-top: var(--space-4); padding: var(--space-3) var(--space-5); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; text-decoration: none; }
  .note { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .note code { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
</style>

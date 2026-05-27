<script lang="ts">
  import { page } from '$app/state';
  const sessionId = $derived(page.url.searchParams.get('session_id') ?? '');
</script>

<main class="centred">
  <h1>Payment received — check your email</h1>
  <p>
    Thanks! Stripe has confirmed your payment. We've emailed you a signed download link that
    works for 24 hours.
  </p>
  {#if sessionId}
    <p class="muted">Stripe session: <code>{sessionId}</code></p>
  {/if}
  <p><a href="/">Back to the storefront</a></p>
</main>

<style>
  .centred {
    max-width: 36rem;
    margin: var(--space-8) auto;
    padding: 0 var(--space-4);
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .muted {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  code {
    background: var(--color-surface);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
  }
</style>

<script lang="ts">
  import type { PageData } from './$types';
  import { checkoutApi, ApiCallError } from '$lib/api';

  let { data }: { data: PageData } = $props();

  let email = $state('');
  let busy = $state<string | null>(null); // product id currently in checkout
  let error = $state<string | null>(null);

  function formatPrice(cents: number, currency: string): string {
    try {
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: currency.toUpperCase()
      }).format(cents / 100);
    } catch {
      return `${(cents / 100).toFixed(2)} ${currency.toUpperCase()}`;
    }
  }

  async function buy(productId: string) {
    error = null;
    if (!email.trim()) {
      error = 'Please enter your email so we can send your download link.';
      return;
    }
    busy = productId;
    try {
      const resp = await checkoutApi.start(window.fetch.bind(window), email.trim(), productId);
      // The browser leaves our site for Stripe's hosted checkout. In test
      // mode the URL points at https://checkout.stripe.com/test/...
      window.location.assign(resp.url);
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'checkout failed';
      busy = null;
    }
  }
</script>

<header class="page-header">
  <h1>Digital Storefront</h1>
  <p class="lead">
    Buy a digital download, get a signed delivery link by email. Payment is handled by Stripe
    Checkout — we never see your card.
  </p>
</header>

<main class="storefront">
  <section class="email-section" aria-labelledby="email-label">
    <h2 id="email-label">Where should we send your download?</h2>
    <label for="email">Email</label>
    <input
      id="email"
      type="email"
      bind:value={email}
      placeholder="you@example.com"
      autocomplete="email"
      required
    />
  </section>

  {#if data.error}
    <p class="error" role="alert">Couldn't load products: {data.error}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <h2>Products</h2>
  {#if data.products.length === 0}
    <p class="empty">No products are listed yet. Check back soon.</p>
  {:else}
    <ul class="product-grid">
      {#each data.products as product (product.id)}
        <li class="product-card">
          <h3>{product.name}</h3>
          <p class="sku">SKU {product.sku}</p>
          {#if product.description}
            <p class="desc">{product.description}</p>
          {/if}
          <p class="price">{formatPrice(product.price_cents, product.currency)}</p>
          <button
            type="button"
            class="primary"
            onclick={() => buy(product.id)}
            disabled={busy === product.id}
          >
            {busy === product.id ? 'Redirecting…' : 'Buy'}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .page-header {
    max-width: 60rem;
    margin: var(--space-6) auto var(--space-4);
    padding: 0 var(--space-4);
  }
  .lead {
    color: var(--color-fg-muted);
    margin-top: var(--space-2);
  }
  .storefront {
    max-width: 60rem;
    margin: 0 auto var(--space-8);
    padding: 0 var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .email-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    display: grid;
    gap: var(--space-2);
  }
  input[type='email'] {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    width: 100%;
  }
  .product-grid {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--space-4);
    grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
  }
  .product-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .sku {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .price {
    margin-top: auto;
    font-weight: 600;
    font-size: var(--text-lg);
  }
  .primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-weight: 600;
  }
  .primary[disabled] {
    opacity: 0.6;
    cursor: progress;
  }
  .error {
    color: var(--color-danger);
    background: var(--color-danger-bg, transparent);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
  }
  .empty {
    color: var(--color-fg-muted);
  }
</style>

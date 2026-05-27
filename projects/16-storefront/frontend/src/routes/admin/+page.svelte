<script lang="ts">
  import { adminApi, authApi, ApiCallError } from '$lib/api';
  import type { AdminProduct, Order, AdminUser } from '$lib/types';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  let user = $state<AdminUser | null>(null);
  let products = $state<AdminProduct[]>([]);
  let orders = $state<Order[]>([]);
  let error = $state<string | null>(null);

  // New-product form
  let np = $state({ sku: '', name: '', description: '', price_cents: 999, currency: 'usd' });
  let busy = $state(false);

  // Per-product upload state
  let uploadFor = $state<string | null>(null);

  onMount(async () => {
    try {
      user = await authApi.me(window.fetch.bind(window));
    } catch {
      await goto('/admin/login');
      return;
    }
    await reload();
  });

  async function reload() {
    try {
      products = await adminApi.listProducts(window.fetch.bind(window));
      orders = await adminApi.listOrders(window.fetch.bind(window));
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'failed to load';
    }
  }

  async function createProduct(event: Event) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      await adminApi.createProduct(window.fetch.bind(window), {
        ...np,
        price_cents: Number(np.price_cents)
      });
      np = { sku: '', name: '', description: '', price_cents: 999, currency: 'usd' };
      await reload();
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'create failed';
    } finally {
      busy = false;
    }
  }

  async function uploadFile(productId: string, file: File) {
    busy = true;
    error = null;
    try {
      const buf = await file.arrayBuffer();
      const bytes = new Uint8Array(buf);
      let binary = '';
      for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i] ?? 0);
      const b64 = btoa(binary);
      await adminApi.uploadFile(window.fetch.bind(window), productId, file.name, b64);
      await reload();
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'upload failed';
    } finally {
      busy = false;
      uploadFor = null;
    }
  }

  async function refundOrder(orderId: string) {
    if (!confirm('Refund this order? This will revoke the customer download link.')) return;
    busy = true;
    error = null;
    try {
      await adminApi.refundOrder(window.fetch.bind(window), orderId);
      await reload();
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'refund failed';
    } finally {
      busy = false;
    }
  }

  async function logout() {
    await authApi.logout(window.fetch.bind(window));
    await goto('/admin/login');
  }

  function fmtPrice(c: number, cur: string): string {
    try {
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: cur.toUpperCase()
      }).format(c / 100);
    } catch {
      return `${(c / 100).toFixed(2)} ${cur.toUpperCase()}`;
    }
  }
</script>

<header class="admin-header">
  <h1>Admin</h1>
  {#if user}
    <div class="who">
      <span>Signed in as {user.email}</span>
      <button type="button" class="ghost" onclick={logout}>Sign out</button>
    </div>
  {/if}
</header>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<section class="admin-section">
  <h2>Add product</h2>
  <form onsubmit={createProduct} class="grid">
    <label>
      SKU
      <input type="text" bind:value={np.sku} required />
    </label>
    <label>
      Name
      <input type="text" bind:value={np.name} required />
    </label>
    <label class="full">
      Description
      <textarea bind:value={np.description} rows="2"></textarea>
    </label>
    <label>
      Price (cents)
      <input type="number" bind:value={np.price_cents} min="1" required />
    </label>
    <label>
      Currency
      <input type="text" bind:value={np.currency} maxlength="3" required />
    </label>
    <div class="full">
      <button type="submit" class="primary" disabled={busy}>Create</button>
    </div>
  </form>
</section>

<section class="admin-section">
  <h2>Products ({products.length})</h2>
  {#if products.length === 0}
    <p class="muted">No products yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>SKU</th>
          <th>Name</th>
          <th>Price</th>
          <th>Active</th>
          <th>File</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each products as p (p.id)}
          <tr>
            <td>{p.sku}</td>
            <td>{p.name}</td>
            <td>{fmtPrice(p.price_cents, p.currency)}</td>
            <td>{p.active ? 'yes' : 'no'}</td>
            <td>{p.file_name ?? '— no file —'}</td>
            <td>
              {#if uploadFor === p.id}
                <input
                  type="file"
                  onchange={(e) => {
                    const f = (e.currentTarget as HTMLInputElement).files?.[0];
                    if (f) uploadFile(p.id, f);
                  }}
                  aria-label="Choose product file"
                />
              {:else}
                <button
                  type="button"
                  class="ghost"
                  onclick={() => (uploadFor = p.id)}
                  disabled={busy}>Upload file</button
                >
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<section class="admin-section">
  <h2>Orders ({orders.length})</h2>
  {#if orders.length === 0}
    <p class="muted">No orders yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Created</th>
          <th>Email</th>
          <th>Status</th>
          <th>Amount</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each orders as o (o.id)}
          <tr>
            <td>{new Date(o.created_at).toLocaleString()}</td>
            <td>{o.customer_email}</td>
            <td>{o.status}</td>
            <td>{fmtPrice(o.amount_cents, o.currency)}</td>
            <td>
              {#if o.status === 'fulfilled' || o.status === 'paid'}
                <button
                  type="button"
                  class="danger"
                  onclick={() => refundOrder(o.id)}
                  disabled={busy}>Refund</button
                >
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .admin-header {
    max-width: 70rem;
    margin: var(--space-6) auto var(--space-3);
    padding: 0 var(--space-4);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .who {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .admin-section {
    max-width: 70rem;
    margin: 0 auto var(--space-5);
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .grid {
    display: grid;
    gap: var(--space-2);
    grid-template-columns: repeat(2, 1fr);
  }
  .grid label {
    display: grid;
    gap: var(--space-1);
  }
  .grid label.full,
  .grid .full {
    grid-column: 1 / -1;
  }
  input,
  textarea {
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    width: 100%;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    padding: var(--space-2);
    border-bottom: 1px solid var(--color-border);
  }
  .primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-weight: 600;
  }
  .ghost {
    color: var(--color-accent);
    padding: var(--space-1) var(--space-2);
  }
  .danger {
    background: var(--color-danger, #c0392b);
    color: white;
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
  }
  .error {
    color: var(--color-danger);
    max-width: 70rem;
    margin: 0 auto;
    padding: var(--space-2) var(--space-4);
  }
  .muted {
    color: var(--color-fg-muted);
  }
</style>

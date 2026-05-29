<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>Tenants — Help Desk</title></svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/tenants">Help Desk</a>
  <span class="who">{data.user?.name || data.user?.email}</span>
  <a class="ghost" href="/logout">Sign out</a>
</nav>

<main>
  <h1>Tenants</h1>
  <p class="lead">Each tenant is RLS-isolated. Your tickets in one can't leak into another.</p>

  <form
    method="POST"
    action="?/create"
    class="create"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => { submitting = false; await update(); };
    }}
  >
    <label class="field">
      <span>Slug</span>
      <input name="slug" required pattern={'[a-z0-9-]{2,40}'} value={(form?.slug as string) ?? ''} />
    </label>
    <label class="field">
      <span>Name</span>
      <input name="name" required value={(form?.name as string) ?? ''} />
    </label>
    <button type="submit" class="primary" disabled={submitting}>{submitting ? '…' : 'Create tenant'}</button>
    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error as string}</p>
    {/if}
  </form>

  {#if data.tenants.length === 0}
    <p class="empty">No tenants yet. Create one above to get started.</p>
  {:else}
    <ul>
      {#each data.tenants as t (t.id)}
        <li>
          <a class="card" href={`/t/${t.slug}`}>
            <span class="name">{t.name}</span>
            <span class="meta">/{t.slug} · {t.role}</span>
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
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .create { display: grid; grid-template-columns: 1fr 2fr auto; gap: var(--space-3); align-items: end; padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input { padding: var(--space-2) var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .error { grid-column: 1 / -1; color: var(--color-danger); font-size: var(--text-sm); }
  .empty { color: var(--color-fg-muted); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .card { display: grid; gap: var(--space-1); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; }
  .name { font-weight: 600; }
  .meta { color: var(--color-fg-muted); font-size: var(--text-sm); }
  @media (max-width: 640px) { .create { grid-template-columns: 1fr; } }
</style>

<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let submitting = $state(false);
  // svelte-ignore state_referenced_locally
  const detail = data.detail;
</script>

<svelte:head><title>{detail.course.title} — Marketplace</title></svelte:head>

<article>
  <a class="back" href="/">← All courses</a>
  <header>
    <h1>{detail.course.title}</h1>
    <p class="instructor">by {detail.course.instructor_name}</p>
    {#if detail.course.summary}<p class="summary">{detail.course.summary}</p>{/if}
  </header>

  <section aria-labelledby="lessons-h">
    <h2 id="lessons-h">Lessons</h2>
    {#if detail.lessons.length === 0}
      <p class="empty">No lessons posted yet.</p>
    {:else}
      <ol>
        {#each detail.lessons as l (l.id)}
          <li>{l.title}{#if l.duration_seconds} · {Math.round(l.duration_seconds / 60)}m{/if}</li>
        {/each}
      </ol>
    {/if}
  </section>

  <aside class="buy">
    <p class="price">${(detail.course.price_cents / 100).toFixed(2)} {detail.course.currency.toUpperCase()}</p>
    {#if detail.enrolled}
      <p class="ok">You're enrolled. Lessons unlock above.</p>
    {:else}
      <form
        method="POST"
        action="?/buy"
        use:enhance={() => {
          submitting = true;
          return async ({ update }) => { submitting = false; await update(); };
        }}
      >
        <button type="submit" class="primary" disabled={submitting}>
          {submitting ? 'Opening Stripe…' : 'Buy course'}
        </button>
        {#if form && 'error' in form && form.error}
          <p class="error" role="alert">{form.error as string}</p>
        {/if}
      </form>
      <p class="note">10% platform fee. Refundable within 14 days.</p>
    {/if}
  </aside>
</article>

<style>
  article { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  .back { color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .instructor, .summary, .note { color: var(--color-fg-muted); font-size: var(--text-sm); }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  ol { padding-left: var(--space-5); }
  .empty { color: var(--color-fg-muted); }
  .buy { padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-align: center; }
  .price { font-size: var(--text-2xl); font-weight: 800; color: var(--color-accent); }
  .primary { padding: var(--space-3) var(--space-5); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .primary:disabled { opacity: 0.5; }
  .ok { color: var(--color-success, var(--color-accent)); margin-top: var(--space-2); }
  .error { color: var(--color-danger); margin-top: var(--space-2); }
</style>

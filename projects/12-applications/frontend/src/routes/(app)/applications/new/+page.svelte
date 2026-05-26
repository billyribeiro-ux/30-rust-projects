<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import { STATUS_LABEL, type AppStatus } from '$lib/types';

  let { form }: PageProps = $props();
  let submitting = $state(false);

  const STATUSES: AppStatus[] = ['wishlist', 'applied', 'screening', 'interview', 'offer'];
</script>

<svelte:head><title>New application</title></svelte:head>

<main>
  <header>
    <h1>Add application</h1>
    <a class="back" href="/applications">← Back</a>
  </header>

  <form
    method="POST"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => {
        submitting = false;
        await update();
      };
    }}
  >
    <div class="row two">
      <label class="field">
        <span>Company <em>*</em></span>
        <input type="text" name="company" required maxlength="200" value={(form?.company as string | undefined) ?? ''} />
      </label>
      <label class="field">
        <span>Role <em>*</em></span>
        <input type="text" name="role" required maxlength="200" value={(form?.role as string | undefined) ?? ''} />
      </label>
    </div>
    <label class="field">
      <span>Location</span>
      <input type="text" name="location" value={(form?.location as string | undefined) ?? ''} />
    </label>
    <label class="field">
      <span>Status</span>
      <select name="status">
        {#each STATUSES as s (s)}
          <option value={s}>{STATUS_LABEL[s]}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span>Job posting URL</span>
      <input type="url" name="job_url" placeholder="https://..." />
    </label>
    <label class="field">
      <span>Notes</span>
      <textarea name="notes" rows="3"></textarea>
    </label>

    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}

    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Saving…' : 'Save'}
    </button>
  </form>
</main>

<style>
  main { max-width: var(--container-md); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  header { display: flex; justify-content: space-between; align-items: baseline; }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .back { color: var(--color-fg-muted); font-size: var(--text-sm); }
  form { display: grid; gap: var(--space-4); padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .row.two { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field > span em { font-style: normal; color: var(--color-danger); }
  .field input, .field textarea, .field select {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-family: var(--font-sans);
    resize: vertical;
  }
  .primary {
    justify-self: start;
    padding: var(--space-3) var(--space-5);
    background: var(--color-accent); color: var(--color-accent-fg);
    border-radius: var(--radius-md); font-weight: 600;
  }
  .primary:hover:not(:disabled) { background: var(--color-accent-hover); }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md); font-size: var(--text-sm);
  }
  @media (min-width: 768px) {
    .row.two { grid-template-columns: 1fr 1fr; }
  }
</style>

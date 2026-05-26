<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>New contact — Contacts</title></svelte:head>

<main>
  <header>
    <h1>Add contact</h1>
    <a class="back" href="/contacts">← Back to contacts</a>
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
        <span>Name <em>*</em></span>
        <input type="text" name="name" required maxlength="200" value={(form?.name as string | undefined) ?? ''} />
      </label>
      <label class="field">
        <span>Company</span>
        <input type="text" name="company" maxlength="200" value={(form?.company as string | undefined) ?? ''} />
      </label>
    </div>
    <div class="row two">
      <label class="field">
        <span>Email</span>
        <input type="email" name="email" value={(form?.email as string | undefined) ?? ''} />
      </label>
      <label class="field">
        <span>Phone</span>
        <input type="tel" name="phone" value={(form?.phone as string | undefined) ?? ''} />
      </label>
    </div>
    <label class="field">
      <span>Notes</span>
      <textarea name="notes" rows="4">{(form?.notes as string | undefined) ?? ''}</textarea>
    </label>
    <label class="field">
      <span>Tags <em>(comma- or space-separated)</em></span>
      <input type="text" name="tags" placeholder="work friend important" value={(form?.tagsRaw as string | undefined) ?? ''} />
    </label>

    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}

    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Saving…' : 'Save contact'}
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
  .field input, .field textarea {
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
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
  }
  .primary:hover:not(:disabled) { background: var(--color-accent-hover); }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  @media (min-width: 768px) {
    .row.two { grid-template-columns: 1fr 1fr; }
  }
</style>

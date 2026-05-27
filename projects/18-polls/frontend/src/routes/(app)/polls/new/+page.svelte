<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();

  let submitting = $state(false);
  let options = $state<string[]>(['', '']);

  function addOption() {
    if (options.length < 16) options.push('');
  }
  function removeOption(i: number) {
    if (options.length > 2) options.splice(i, 1);
  }

  // Suggest a random short slug.
  function randomSlug() {
    return Math.random().toString(36).slice(2, 8);
  }
  let slug = $state(form?.slug?.toString() ?? randomSlug());
</script>

<svelte:head><title>New poll</title></svelte:head>

<h1>New poll</h1>
<p class="lead">Give it a question and a few options. You'll get a join URL and QR code.</p>

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
  <label class="field">
    <span>Slug (URL fragment, 4-32 chars)</span>
    <input
      type="text"
      name="slug"
      bind:value={slug}
      required
      minlength="4"
      maxlength="32"
      pattern="[a-zA-Z0-9_\-]+"
      autocomplete="off"
    />
  </label>

  <label class="field">
    <span>Question</span>
    <input
      type="text"
      name="question"
      required
      maxlength="500"
      placeholder="What should we ship next quarter?"
      value={form?.question?.toString() ?? ''}
    />
  </label>

  <fieldset class="options">
    <legend>Options ({options.length})</legend>
    {#each options as opt, i (i)}
      <div class="opt-row">
        <input
          type="text"
          name={`option_${i}`}
          placeholder={`Option ${i + 1}`}
          required
          maxlength="200"
          bind:value={options[i]}
        />
        <button
          type="button"
          class="ghost"
          aria-label={`Remove option ${i + 1}`}
          onclick={() => removeOption(i)}
          disabled={options.length <= 2}
        >×</button>
      </div>
    {/each}
    <button type="button" class="ghost add" onclick={addOption} disabled={options.length >= 16}>
      + Add option
    </button>
  </fieldset>

  {#if form && 'error' in form && form.error}
    <p class="error" role="alert">{form.error}</p>
  {/if}

  <button type="submit" class="primary" disabled={submitting}>
    {submitting ? 'Creating…' : 'Create poll'}
  </button>
</form>

<style>
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin-bottom: var(--space-5); }
  form { display: grid; gap: var(--space-4); max-width: 36rem; }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  input[type="text"] {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    width: 100%;
  }
  .options {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    display: grid;
    gap: var(--space-2);
  }
  legend { padding: 0 var(--space-2); font-size: var(--text-sm); color: var(--color-fg-muted); }
  .opt-row { display: flex; gap: var(--space-2); }
  .ghost {
    padding: var(--space-2) var(--space-3);
    background: transparent;
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .ghost:hover:not(:disabled) { border-color: var(--color-border-strong); }
  .ghost:disabled { opacity: 0.4; cursor: not-allowed; }
  .ghost.add { justify-self: start; }
  .primary {
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    justify-self: start;
  }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
</style>

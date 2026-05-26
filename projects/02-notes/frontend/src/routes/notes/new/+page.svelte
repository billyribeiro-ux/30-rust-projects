<script lang="ts">
  import { enhance } from '$app/forms';
  import { FloppyDisk, Eye, PencilSimple } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { preview } from '$lib/markdown';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();

  // bound to the inputs; user's typed values are preserved across
  // form-action error round-trips because the component does not remount.
  let title = $state('');
  let body_md = $state('');
  let mode = $state<'edit' | 'preview'>('edit');
  let submitting = $state(false);

  const previewHtml = $derived(mode === 'preview' ? preview(body_md) : '');
</script>

<svelte:head>
  <title>New note — Notes</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  <header>
    <h1>New note</h1>
    <div class="tabs" role="tablist" aria-label="Editor mode">
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'edit'}
        class:active={mode === 'edit'}
        onclick={() => (mode = 'edit')}
      >
        <Icon icon={PencilSimple} size={16} weight="bold" />
        <span>Edit</span>
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'preview'}
        class:active={mode === 'preview'}
        onclick={() => (mode = 'preview')}
      >
        <Icon icon={Eye} size={16} weight="bold" />
        <span>Preview</span>
      </button>
    </div>
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
    <label class="field">
      <span>Title</span>
      <input
        type="text"
        name="title"
        bind:value={title}
        placeholder="A short, descriptive title"
        autocomplete="off"
        maxlength="200"
        required
      />
    </label>

    {#if mode === 'edit'}
      <label class="field">
        <span>Body (Markdown)</span>
        <textarea
          name="body_md"
          bind:value={body_md}
          rows="14"
          placeholder="# Your idea&#10;&#10;Write it down before you lose it…"
        ></textarea>
      </label>
    {:else}
      <input type="hidden" name="body_md" value={body_md} />
      <section class="preview" aria-label="Markdown preview">
        {#if body_md.trim().length === 0}
          <p class="hint">Nothing to preview yet.</p>
        {:else}
          <div class="prose">{@html previewHtml}</div>
        {/if}
      </section>
    {/if}

    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}

    <div class="actions">
      <button
        type="submit"
        class="primary"
        disabled={submitting || title.trim().length === 0}
      >
        <Icon icon={FloppyDisk} size={18} weight="bold" />
        <span>{submitting ? 'Saving…' : 'Save note'}</span>
      </button>
    </div>
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  h1 {
    font-size: var(--text-3xl);
    font-weight: 700;
    line-height: var(--leading-tight);
  }

  .tabs {
    display: inline-flex;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    padding: 2px;
  }

  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }

  .tabs button.active {
    background: var(--color-bg);
    color: var(--color-fg);
    box-shadow: var(--shadow-sm);
  }

  form {
    display: grid;
    gap: var(--space-4);
  }

  .field {
    display: grid;
    gap: var(--space-2);
  }

  .field span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }

  input[type='text'],
  textarea {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    transition: border-color var(--duration-fast) var(--ease-out);
  }

  textarea {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
    resize: vertical;
  }

  input[type='text']:focus-visible,
  textarea:focus-visible {
    border-color: var(--color-accent);
    outline-offset: 0;
  }

  .preview {
    min-height: 280px;
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .prose :global(h1) {
    font-size: var(--text-2xl);
    margin-bottom: var(--space-3);
  }
  .prose :global(h2) {
    font-size: var(--text-xl);
    margin: var(--space-4) 0 var(--space-2);
  }
  .prose :global(p) {
    margin-bottom: var(--space-3);
    line-height: var(--leading-relaxed);
  }
  .prose :global(ul),
  .prose :global(ol) {
    padding-left: 1.5em;
    margin-bottom: var(--space-3);
  }
  .prose :global(code) {
    font-family: var(--font-mono);
    background: var(--color-bg-sunken);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    font-size: 0.9em;
  }
  .prose :global(pre) {
    padding: var(--space-3);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    overflow-x: auto;
  }

  .hint {
    color: var(--color-fg-muted);
    font-style: italic;
  }

  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
      gap: var(--space-6);
    }
  }
</style>

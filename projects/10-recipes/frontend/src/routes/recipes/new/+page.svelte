<script lang="ts">
  import { enhance } from '$app/forms';
  import { ArrowLeft, FloppyDisk } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
</script>

<svelte:head>
  <title>New recipe — Recipes</title>
</svelte:head>

<main>
  <a href="/" class="back"><Icon icon={ArrowLeft} size={16} /><span>Back</span></a>
  <h1>New recipe</h1>
  <p class="hint">Create the recipe first — you'll add photos on the edit page.</p>

  <form method="POST" use:enhance class="grid">
    <label class="field">
      <span>Title</span>
      <input
        type="text"
        name="title"
        required
        maxlength="200"
        value={form?.title ?? ''}
        autocomplete="off"
      />
    </label>

    <label class="field">
      <span>Description (optional)</span>
      <textarea
        name="description"
        rows="2"
        maxlength="2000"
        value={form?.description ?? ''}
      ></textarea>
    </label>

    <label class="field">
      <span>Ingredients (one per line)</span>
      <textarea
        name="ingredients"
        rows="6"
        placeholder="2 cups flour&#10;1 tsp salt&#10;..."
        value={form?.ingredients ?? ''}
      ></textarea>
    </label>

    <label class="field">
      <span>Instructions (one per line, in order)</span>
      <textarea
        name="instructions"
        rows="6"
        placeholder="Preheat oven to 200C.&#10;Mix dry ingredients.&#10;..."
        value={form?.instructions ?? ''}
      ></textarea>
    </label>

    <div class="row three">
      <label class="field">
        <span>Prep (min)</span>
        <input type="number" name="prep_minutes" min="0" max="10080" />
      </label>
      <label class="field">
        <span>Cook (min)</span>
        <input type="number" name="cook_minutes" min="0" max="10080" />
      </label>
      <label class="field">
        <span>Servings</span>
        <input type="number" name="servings" min="1" max="1000" />
      </label>
    </div>

    {#if form?.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}

    <button type="submit" class="primary">
      <Icon icon={FloppyDisk} size={16} weight="bold" />
      <span>Create & add photos</span>
    </button>
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-4);
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .back:hover {
    color: var(--color-fg);
  }
  h1 {
    font-size: var(--text-2xl);
    font-weight: 800;
  }
  .hint {
    color: var(--color-fg-muted);
  }
  .grid {
    display: grid;
    gap: var(--space-3);
  }
  .field {
    display: grid;
    gap: var(--space-1);
  }
  .field > span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .field input,
  .field textarea {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .field textarea {
    font-family: var(--font-sans);
    resize: vertical;
  }
  .row.three {
    display: grid;
    gap: var(--space-3);
  }
  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    justify-self: start;
  }
  .primary:hover {
    background: var(--color-accent-hover);
  }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  @media (min-width: 768px) {
    .row.three {
      grid-template-columns: 1fr 1fr 1fr;
    }
  }
</style>

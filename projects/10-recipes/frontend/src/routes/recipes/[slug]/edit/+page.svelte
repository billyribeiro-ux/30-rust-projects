<script lang="ts">
  import { enhance } from '$app/forms';
  import { ArrowLeft, FloppyDisk, Trash, UploadSimple } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { uploadPhoto, removePhoto } from './photos.remote';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  const recipe = $derived(data.recipe);

  // Local state hydrated from the loaded recipe. We deliberately snapshot
  // here — the form is the user's working copy, separate from the
  // server's truth (`data.recipe`). The svelte-ignore acknowledges
  // we're reading `recipe` outside a `$derived`/`$effect`, which is
  // the right call for "one-time initialise from props".
  /* svelte-ignore state_referenced_locally */
  let title = $state(recipe.title);
  /* svelte-ignore state_referenced_locally */
  let description = $state(recipe.description);
  /* svelte-ignore state_referenced_locally */
  let ingredients = $state(recipe.ingredients.join('\n'));
  /* svelte-ignore state_referenced_locally */
  let instructions = $state(recipe.instructions.join('\n'));
  /* svelte-ignore state_referenced_locally */
  let prep = $state<number | ''>(recipe.prep_minutes ?? '');
  /* svelte-ignore state_referenced_locally */
  let cook = $state<number | ''>(recipe.cook_minutes ?? '');
  /* svelte-ignore state_referenced_locally */
  let servings = $state<number | ''>(recipe.servings ?? '');

  // Drag & drop area state — for the file-upload lesson.
  let dragOver = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();
  let uploadForm: HTMLFormElement | undefined = $state();

  function pickFiles() {
    fileInput?.click();
  }
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0 || !fileInput) return;
    // Assign the dropped file to the hidden input then submit.
    const dt = new DataTransfer();
    dt.items.add(files[0] as File);
    fileInput.files = dt.files;
    queueMicrotask(() => uploadForm?.requestSubmit());
  }

  async function handleRemovePhoto(image_id: string) {
    if (!confirm('Delete this photo?')) return;
    await removePhoto({ image_id, recipe_id: recipe.id });
  }
</script>

<svelte:head>
  <title>Edit {recipe.title} — Recipes</title>
</svelte:head>

<main>
  <a href="/recipes/{recipe.slug}" class="back">
    <Icon icon={ArrowLeft} size={16} />
    <span>Back to recipe</span>
  </a>
  <h1>Edit recipe</h1>

  <section class="panel" aria-label="Photos">
    <h2>Photos</h2>

    <!--
      File upload UX.
      The <form> uses SvelteKit remote-function progressive enhancement
      via `{...uploadPhoto}`. The spread sets `method`, `enctype`, and
      `onsubmit` so the form submits without a full navigation and the
      typed result lands in `uploadPhoto.result`.
    -->
    <form
      bind:this={uploadForm}
      {...uploadPhoto}
      class="dropzone"
      class:dragover={dragOver}
      ondragover={(e) => {
        e.preventDefault();
        dragOver = true;
      }}
      ondragleave={() => (dragOver = false)}
      ondrop={onDrop}
    >
      <input type="hidden" name="recipe_id" value={recipe.id} />
      <input
        bind:this={fileInput}
        type="file"
        name="file"
        accept="image/jpeg,image/png,image/webp"
        onchange={() => queueMicrotask(() => uploadForm?.requestSubmit())}
        hidden
      />
      <button type="button" onclick={pickFiles} class="drop-target">
        <Icon icon={UploadSimple} size={24} weight="bold" />
        <span class="drop-label">Drop a photo, or click to pick</span>
        <span class="drop-hint">JPEG / PNG / WebP, up to 10 MB</span>
      </button>
      {#if uploadPhoto.result && uploadPhoto.result.ok === false}
        <p class="error" role="alert">{uploadPhoto.result.error}</p>
      {/if}
    </form>

    {#if recipe.images.length > 0}
      <ul class="thumbs">
        {#each recipe.images as img (img.id)}
          <li>
            <img src={img.thumb_url} alt="" loading="lazy" width="200" height="150" />
            <button
              type="button"
              class="remove-photo"
              aria-label="Delete photo"
              onclick={() => handleRemovePhoto(img.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">No photos yet. Drop one above.</p>
    {/if}
  </section>

  <section class="panel" aria-label="Details">
    <h2>Details</h2>
    <form method="POST" action="?/save" use:enhance class="grid">
      <label class="field">
        <span>Title</span>
        <input type="text" name="title" required maxlength="200" bind:value={title} />
      </label>
      <label class="field">
        <span>Description</span>
        <textarea name="description" rows="2" maxlength="2000" bind:value={description}></textarea>
      </label>
      <label class="field">
        <span>Ingredients (one per line)</span>
        <textarea name="ingredients" rows="6" bind:value={ingredients}></textarea>
      </label>
      <label class="field">
        <span>Instructions (one per line, in order)</span>
        <textarea name="instructions" rows="6" bind:value={instructions}></textarea>
      </label>
      <div class="row three">
        <label class="field">
          <span>Prep (min)</span>
          <input type="number" name="prep_minutes" min="0" max="10080" bind:value={prep} />
        </label>
        <label class="field">
          <span>Cook (min)</span>
          <input type="number" name="cook_minutes" min="0" max="10080" bind:value={cook} />
        </label>
        <label class="field">
          <span>Servings</span>
          <input type="number" name="servings" min="1" max="1000" bind:value={servings} />
        </label>
      </div>
      {#if form?.error}
        <p class="error" role="alert">{form.error}</p>
      {/if}
      {#if form?.success}
        <p class="success" role="status">Saved.</p>
      {/if}
      <button type="submit" class="primary">
        <Icon icon={FloppyDisk} size={16} weight="bold" />
        <span>Save</span>
      </button>
    </form>
  </section>

  <section class="panel danger-panel" aria-label="Danger zone">
    <h2>Danger zone</h2>
    <form
      method="POST"
      action="?/remove"
      onsubmit={(e) => {
        if (!confirm(`Delete "${recipe.title}"? Photos and ratings will be removed too.`))
          e.preventDefault();
      }}
    >
      <button type="submit" class="danger">
        <Icon icon={Trash} size={16} weight="bold" />
        <span>Delete this recipe</span>
      </button>
    </form>
  </section>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
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
  .panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-4);
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .dropzone {
    border: 2px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    background: var(--color-bg-sunken);
    transition: border-color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out);
  }
  .dropzone.dragover {
    border-color: var(--color-accent);
    background: hsl(231 84% 56% / 0.05);
  }
  .drop-target {
    width: 100%;
    display: grid;
    place-items: center;
    gap: var(--space-1);
    padding: var(--space-6) var(--space-3);
    color: var(--color-fg-muted);
  }
  .drop-label {
    font-weight: 600;
    color: var(--color-fg);
  }
  .drop-hint {
    font-size: var(--text-xs);
  }
  .thumbs {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  }
  .thumbs li {
    position: relative;
    aspect-ratio: 4 / 3;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--color-bg-sunken);
  }
  .thumbs img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .remove-photo {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: hsl(0 72% 42%);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    opacity: 0.85;
  }
  .remove-photo:hover {
    opacity: 1;
  }
  .empty {
    color: var(--color-fg-muted);
    padding: var(--space-3);
    text-align: center;
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
  .success {
    padding: var(--space-3) var(--space-4);
    color: var(--color-success);
    background: hsl(142 71% 35% / 0.08);
    border: 1px solid hsl(142 71% 35% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  .danger-panel {
    border-color: hsl(0 72% 51% / 0.4);
  }
  .danger {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    color: hsl(0 72% 42%);
    border: 1px solid hsl(0 72% 51% / 0.4);
    border-radius: var(--radius-md);
    font-weight: 600;
    background: var(--color-bg);
  }
  .danger:hover {
    background: hsl(0 72% 51% / 0.08);
  }
  @media (min-width: 768px) {
    .row.three {
      grid-template-columns: 1fr 1fr 1fr;
    }
  }
</style>

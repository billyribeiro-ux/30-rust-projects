<!--
  Reusable RecipeCard SNIPPET.

  Svelte 5 lets us declare a `{#snippet}` inside a `<script module>`
  block by writing it at the top of a `.svelte` file and exporting it
  through the module script. Snippets are first-class values, so any
  other file can import them and `{@render card(recipe)}`.

  Why a snippet and not a component?
  - A component re-mounts on prop change; a snippet renders inline like
    a function call. For a static card with no internal state, the
    snippet is lighter and reads as a more honest "this is a piece of
    template, not a black box".
  - This is project 10's required "snippets" lesson. The pattern is
    used twice: the home grid AND a related-recipes section.
-->

<script lang="ts" module>
  import type { RecipeSummary } from '$lib/types';

  // Re-export the snippet from a `.svelte` module-script. Note we
  // declare it OUTSIDE the instance script so consumers can
  // `import { card } from '$lib/components/RecipeCard.svelte';`.
  export { card };
</script>

<script lang="ts">
  // The instance script can be empty — this file is purely a snippet
  // host. Svelte 5 still requires at least the module-level export
  // shape.
</script>

{#snippet card(recipe: RecipeSummary)}
  <article class="card" data-testid="recipe-card" data-slug={recipe.slug}>
    <a href="/recipes/{recipe.slug}" class="link">
      <div class="cover">
        {#if recipe.cover_thumb_url}
          <img
            src={recipe.cover_thumb_url}
            alt=""
            loading="lazy"
            width="400"
            height="300"
          />
        {:else}
          <div class="cover-fallback" aria-hidden="true">
            <span>{recipe.title.slice(0, 2).toUpperCase()}</span>
          </div>
        {/if}
      </div>
      <div class="meta">
        <h2>{recipe.title}</h2>
        {#if recipe.description}
          <p class="desc">{recipe.description}</p>
        {/if}
        <p class="stats">
          {#if recipe.prep_minutes != null}
            <span>{recipe.prep_minutes + (recipe.cook_minutes ?? 0)} min</span>
          {/if}
          {#if recipe.servings != null}
            <span aria-label="servings">{recipe.servings} servings</span>
          {/if}
        </p>
      </div>
    </a>
  </article>
{/snippet}

<style>
  .card {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .card:hover {
    border-color: var(--color-border-strong);
  }
  .link {
    display: grid;
    color: inherit;
    text-decoration: none;
  }
  .cover {
    aspect-ratio: 4 / 3;
    background: var(--color-bg-sunken);
    overflow: hidden;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--color-fg-muted);
    font-weight: 700;
    font-size: var(--text-2xl);
  }
  .meta {
    padding: var(--space-3) var(--space-4);
    display: grid;
    gap: var(--space-1);
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: 700;
    line-height: var(--leading-tight);
  }
  .desc {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .stats {
    display: inline-flex;
    gap: var(--space-3);
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
    margin-top: var(--space-1);
  }
</style>

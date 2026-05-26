<script lang="ts">
  import { ForkKnife, Plus } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { card } from '$lib/components/RecipeCard.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const recipes = $derived(data.recipes);

  // RELATED-RECIPES section: the 3 most-recently-updated other recipes.
  // Used to demonstrate snippet reuse — we render `{@render card(r)}`
  // in TWO places: the main grid AND this related strip.
  const related = $derived(recipes.slice(0, 3));
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={ForkKnife} size={28} weight="duotone" />
      <h1>Recipes</h1>
    </div>
    <p class="lead">14 screenshots → one cookbook. Add, photograph, share.</p>
    <a href="/recipes/new" class="primary">
      <Icon icon={Plus} size={16} weight="bold" />
      <span>New recipe</span>
    </a>
  </header>

  {#if recipes.length === 0}
    <p class="empty">
      No recipes yet. <a href="/recipes/new">Add your first</a>.
    </p>
  {:else}
    <section aria-label="All recipes">
      <h2 class="section-title">All recipes</h2>
      <ul class="grid">
        {#each recipes as recipe (recipe.id)}
          <li>
            <!--
              First render of the snippet: the main grid.
              `card` is imported from RecipeCard.svelte's module script
              and passed `recipe` as its single argument.
            -->
            {@render card(recipe)}
          </li>
        {/each}
      </ul>
    </section>

    {#if related.length > 0}
      <section aria-label="Recently added" class="related">
        <h2 class="section-title">Recently added</h2>
        <ul class="grid related-grid">
          {#each related as recipe (recipe.id)}
            <li>
              <!-- Second render: the related-recipes strip. SAME snippet. -->
              {@render card(recipe)}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
</main>

<style>
  main {
    max-width: var(--container-lg);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }
  header {
    display: grid;
    gap: var(--space-2);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
  }
  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
  }
  .lead {
    color: var(--color-fg-muted);
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
    margin-top: var(--space-2);
  }
  .primary:hover {
    background: var(--color-accent-hover);
  }
  .section-title {
    font-size: var(--text-lg);
    font-weight: 700;
    margin-bottom: var(--space-3);
  }
  .grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-4);
  }
  .related-grid {
    margin-top: var(--space-3);
  }
  .empty {
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }
  .related {
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-5);
  }
  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }
    .grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @media (min-width: 1024px) {
    .grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>

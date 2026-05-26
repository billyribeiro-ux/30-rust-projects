<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const recipe = $derived(data.recipe);
</script>

<svelte:head>
  <title>{recipe.title} (shared) — Recipes</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  <p class="banner">You're viewing a shared recipe. This link will expire in 24 hours.</p>

  <article class="recipe">
    <header>
      <h1>{recipe.title}</h1>
      {#if recipe.description}<p class="desc">{recipe.description}</p>{/if}
      <dl class="tiles">
        {#if recipe.prep_minutes != null}
          <div><dt>Prep</dt><dd>{recipe.prep_minutes} min</dd></div>
        {/if}
        {#if recipe.cook_minutes != null}
          <div><dt>Cook</dt><dd>{recipe.cook_minutes} min</dd></div>
        {/if}
        {#if recipe.servings != null}
          <div><dt>Serves</dt><dd>{recipe.servings}</dd></div>
        {/if}
      </dl>
    </header>

    {#if recipe.images.length > 0}
      <section class="gallery" aria-label="Photos">
        {#each recipe.images as img (img.id)}
          <figure>
            <img src={img.url} alt="" loading="lazy" width={img.width} height={img.height} />
          </figure>
        {/each}
      </section>
    {/if}

    <section class="panel" aria-label="Ingredients">
      <h2>Ingredients</h2>
      <ul>
        {#each recipe.ingredients as ingredient, i (i)}
          <li>{ingredient}</li>
        {/each}
      </ul>
    </section>

    <section class="panel" aria-label="Instructions">
      <h2>Instructions</h2>
      <ol>
        {#each recipe.instructions as step, i (i)}
          <li>{step}</li>
        {/each}
      </ol>
    </section>
  </article>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }
  .banner {
    padding: var(--space-3);
    background: hsl(231 84% 56% / 0.08);
    border: 1px solid hsl(231 84% 56% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--color-fg);
  }
  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
  }
  .desc {
    color: var(--color-fg-muted);
    margin-top: var(--space-2);
  }
  .tiles {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-top: var(--space-4);
    padding: var(--space-3);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .tiles > div {
    display: grid;
    gap: 2px;
    padding: var(--space-1) var(--space-3);
  }
  .tiles dt {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .tiles dd {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .gallery {
    display: grid;
    gap: var(--space-3);
  }
  .gallery figure {
    overflow: hidden;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-bg-sunken);
    aspect-ratio: 4 / 3;
  }
  .gallery img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: 700;
  }
  .panel ul,
  .panel ol {
    padding-left: var(--space-5);
    display: grid;
    gap: var(--space-2);
  }
  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }
    .gallery {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>

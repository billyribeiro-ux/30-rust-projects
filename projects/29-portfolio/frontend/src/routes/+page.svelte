<script lang="ts">
  import { heroIntro, revealOnScroll } from '$lib/motion';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let hero = $state<HTMLElement | null>(null);

  $effect(() => {
    if (hero) heroIntro(hero);
  });
</script>

<main>
  <section class="hero" bind:this={hero}>
    <h1>Selected work</h1>
    <p class="lead">Long-form case studies, image-heavy. SSR-first for SEO.</p>
  </section>

  {#if data.error}
    <p class="error" role="alert">{data.error}</p>
  {:else if data.posts.length === 0}
    <p class="empty">No published posts yet. Sign in to the CMS to publish one.</p>
  {:else}
    <ul class="work">
      {#each data.posts as p (p.id)}
        <li
          {@attach (node) => {
            revealOnScroll(node);
          }}
        >
          <a class="card" href={`/work/${p.slug}`}>
            {#if p.hero_image_url}
              <img src={p.hero_image_url} alt="" loading="lazy" />
            {:else}
              <div class="placeholder" aria-hidden="true"></div>
            {/if}
            <h2>{p.title}</h2>
            <p>{p.summary}</p>
            {#if p.published_at}
              <time datetime={p.published_at}>{new Date(p.published_at).toLocaleDateString()}</time>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main { max-width: 1080px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-6); }
  .hero { padding: var(--space-7) 0 var(--space-4); }
  .hero h1 { font-size: clamp(2.5rem, 6vw, 4.5rem); font-weight: 800; letter-spacing: -0.02em; line-height: 1.05; background: linear-gradient(135deg, var(--color-fg), hsl(220 50% 50%)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .hero .lead { color: var(--color-fg-muted); margin-top: var(--space-3); font-size: var(--text-lg); }
  .error, .empty { color: var(--color-fg-muted); }
  .work { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: var(--space-4); }
  .card { display: grid; gap: var(--space-2); padding: var(--space-4); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-decoration: none; color: inherit; transition: transform var(--duration-fast) var(--ease-out); }
  .card:hover { transform: translateY(-4px); }
  .card img, .card .placeholder { width: 100%; aspect-ratio: 3 / 2; border-radius: var(--radius-sm); background: linear-gradient(135deg, hsl(220 30% 30%), hsl(280 40% 40%)); object-fit: cover; }
  .card h2 { font-size: var(--text-xl); font-weight: 700; }
  .card p { color: var(--color-fg-muted); font-size: var(--text-sm); }
  time { color: var(--color-fg-muted); font-size: var(--text-xs); }
</style>

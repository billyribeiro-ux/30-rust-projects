<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  const post = data.post;

  const articleJsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'CreativeWork',
      name: post.title,
      description: post.summary,
      datePublished: post.published_at,
      dateModified: post.updated_at,
      inLanguage: post.locale
    })
  );
</script>

<svelte:head>
  <title>{post.title} — Cinematic Portfolio</title>
  <meta name="description" content={post.summary} />
  <link rel="alternate" hreflang={post.locale} href={`/work/${post.slug}`} />
  {@html `<script type="application/ld+json">${articleJsonLd}</script>`}
</svelte:head>

<article>
  <a class="back" href="/">← All work</a>
  <header>
    <h1>{post.title}</h1>
    {#if post.summary}<p class="summary">{post.summary}</p>{/if}
  </header>
  {#if post.hero_image_url}
    <img class="hero" src={post.hero_image_url} alt="" />
  {/if}
  <div class="body">{@html post.body_html}</div>
</article>

<style>
  article { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  .back { color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  h1 { font-size: var(--text-3xl); font-weight: 800; letter-spacing: -0.02em; line-height: 1.1; }
  .summary { color: var(--color-fg-muted); margin-top: var(--space-2); font-size: var(--text-lg); }
  img.hero { width: 100%; border-radius: var(--radius-md); aspect-ratio: 16 / 9; object-fit: cover; }
  .body { line-height: 1.75; }
  .body :global(h2) { font-size: var(--text-xl); margin-top: var(--space-5); }
  .body :global(p) { margin-top: var(--space-3); }
  .body :global(code) { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
</style>

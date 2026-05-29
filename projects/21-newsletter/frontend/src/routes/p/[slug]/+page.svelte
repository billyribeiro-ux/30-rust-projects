<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  const post = data.post;

  const jsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'Article',
      headline: post.title,
      datePublished: post.published_at,
      description: post.summary,
      isAccessibleForFree: !post.is_gated,
      hasPart: post.is_gated
        ? {
            '@type': 'WebPageElement',
            isAccessibleForFree: false,
            cssSelector: '.paywalled-body'
          }
        : undefined
    })
  );
</script>

<svelte:head>
  <title>{post.title} — Newsletter</title>
  <meta name="description" content={post.summary} />
  {@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<article>
  <header>
    <a class="back" href="/">← All posts</a>
    <h1>{post.title}</h1>
    {#if post.summary}<p class="lead">{post.summary}</p>{/if}
    {#if post.published_at}
      <time datetime={post.published_at}>{new Date(post.published_at).toLocaleDateString()}</time>
    {/if}
  </header>

  <div class={post.paywalled ? 'body paywalled-body' : 'body'}>
    {@html post.body_html}
  </div>

  {#if post.paywalled}
    <aside class="paywall">
      <h2>This post is for Pro subscribers</h2>
      <p>Subscribe on the home page to read the full post.</p>
      <a class="primary" href="/">Subscribe</a>
    </aside>
  {/if}
</article>

<style>
  article { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); }
  .back { color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  h1 { font-size: var(--text-3xl); font-weight: 800; margin-top: var(--space-3); }
  .lead { color: var(--color-fg-muted); margin-top: var(--space-2); }
  time { color: var(--color-fg-muted); font-size: var(--text-sm); display: block; margin-top: var(--space-2); }
  .body { margin-top: var(--space-5); line-height: 1.7; }
  .body :global(h2) { font-size: var(--text-xl); margin-top: var(--space-5); }
  .body :global(p) { margin-top: var(--space-3); }
  .body :global(code) { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
  .paywall { margin-top: var(--space-6); padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); text-align: center; }
  .paywall h2 { font-size: var(--text-xl); font-weight: 700; }
  .paywall p { color: var(--color-fg-muted); margin-top: var(--space-2); margin-bottom: var(--space-3); }
  .primary { display: inline-block; padding: var(--space-3) var(--space-5); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; text-decoration: none; }
</style>

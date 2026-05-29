<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  const article = data.article;

  const articleJsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'TechArticle',
      headline: article.title,
      description: article.summary,
      inLanguage: article.locale,
      datePublished: article.published_at,
      dateModified: article.updated_at
    })
  );
  const faqJsonLd = $derived(
    article.faqs.length > 0
      ? JSON.stringify({
          '@context': 'https://schema.org',
          '@type': 'FAQPage',
          mainEntity: article.faqs.map((f) => ({
            '@type': 'Question',
            name: f.q,
            acceptedAnswer: { '@type': 'Answer', text: f.a }
          }))
        })
      : null
  );
</script>

<svelte:head>
  <title>{article.title} — Knowledge Base</title>
  <meta name="description" content={article.summary} />
  <link rel="alternate" hreflang={article.locale} href={`/a/${article.slug}`} />
  {@html `<script type="application/ld+json">${articleJsonLd}</script>`}
  {#if faqJsonLd}
    {@html `<script type="application/ld+json">${faqJsonLd}</script>`}
  {/if}
</svelte:head>

<article>
  <nav aria-label="Breadcrumb">
    <a href="/">← Back to all</a>
    {#if article.category_slug}
      <span class="sep">·</span>
      <span>{article.category_slug}</span>
    {/if}
  </nav>

  <header>
    <h1>{article.title}</h1>
    {#if article.summary}
      <aside class="answer-block" role="region" aria-label="Quick answer">
        <h2>Quick answer</h2>
        <p>{article.summary}</p>
      </aside>
    {/if}
  </header>

  <div class="body">{@html article.body_html}</div>

  {#if article.faqs.length > 0}
    <section aria-labelledby="faq-h">
      <h2 id="faq-h">FAQ</h2>
      <ul>
        {#each article.faqs as faq, idx (idx)}
          <li>
            <h3>{faq.q}</h3>
            <p>{faq.a}</p>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</article>

<style>
  article { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  nav { color: var(--color-fg-muted); font-size: var(--text-sm); }
  nav a { color: var(--color-fg-muted); text-decoration: none; }
  .sep { margin: 0 var(--space-2); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .answer-block { background: var(--color-bg-elev); border: 1px solid var(--color-accent); border-radius: var(--radius-md); padding: var(--space-4); margin-top: var(--space-4); }
  .answer-block h2 { font-size: var(--text-sm); font-weight: 700; text-transform: uppercase; color: var(--color-accent); letter-spacing: 0.04em; margin-bottom: var(--space-2); }
  .body { line-height: 1.7; }
  .body :global(h2) { font-size: var(--text-xl); margin-top: var(--space-5); }
  .body :global(p) { margin-top: var(--space-3); }
  .body :global(code) { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
  ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  ul h3 { font-weight: 600; }
  ul p { color: var(--color-fg-muted); }
</style>

<script lang="ts">
  import { LockKey, PencilSimple, Globe } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';

  const jsonld = {
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    name: 'Notes',
    description: 'A tiny, fast Markdown notes app with server-side sanitization.',
    applicationCategory: 'Productivity',
    operatingSystem: 'Web',
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' }
  };
</script>

<svelte:head>
  <title>About — Notes</title>
  <meta
    name="description"
    content="Notes is a tiny Markdown app. Server-side sanitization, fast pages, no tracking."
  />
  <link rel="canonical" href="https://notes.example.com/about" />
  <meta property="og:type" content="article" />
  <meta property="og:title" content="About — Notes" />
  <meta
    property="og:description"
    content="Notes is a tiny Markdown app. Server-side sanitization, fast pages, no tracking."
  />
  {@html `<script type="application/ld+json">${JSON.stringify(jsonld)}</` + `script>`}
</svelte:head>

<main>
  <header>
    <h1>Notes</h1>
    <p class="lead">
      A tiny, fast Markdown app that doesn't get in your way. Draft your idea, hit save, share the
      link. Done.
    </p>
  </header>

  <section class="features" aria-label="Features">
    <article>
      <span class="ico"><Icon icon={PencilSimple} size={24} weight="duotone" /></span>
      <h2>Markdown that just works</h2>
      <p>
        Write in plain Markdown. Headings, lists, links, code blocks, tables — rendered the way you
        expect.
      </p>
    </article>

    <article>
      <span class="ico"><Icon icon={LockKey} size={24} weight="duotone" /></span>
      <h2>Safe by default</h2>
      <p>
        Every note is sanitized server-side with <code>ammonia</code> before storage. No script
        tags, no <code>onclick</code>, no <code>javascript:</code> URLs. Ever.
      </p>
    </article>

    <article>
      <span class="ico"><Icon icon={Globe} size={24} weight="duotone" /></span>
      <h2>Shareable links</h2>
      <p>
        Every note gets a clean URL like <code>/notes/your-title</code>. Send the link, it loads
        fast, it indexes well.
      </p>
    </article>
  </section>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-12) var(--space-4);
    display: grid;
    gap: var(--space-12);
  }

  header {
    display: grid;
    gap: var(--space-4);
    text-align: center;
  }

  h1 {
    font-size: var(--text-4xl);
    font-weight: 800;
    line-height: var(--leading-tight);
  }

  .lead {
    font-size: var(--text-lg);
    color: var(--color-fg-muted);
    max-width: 56ch;
    margin-inline: auto;
  }

  .features {
    display: grid;
    gap: var(--space-6);
  }

  article {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    display: grid;
    gap: var(--space-3);
  }

  .ico {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md);
    background: var(--color-bg-sunken);
    color: var(--color-accent);
  }

  h2 {
    font-size: var(--text-xl);
    font-weight: 600;
  }

  article p {
    color: var(--color-fg-muted);
    line-height: var(--leading-relaxed);
  }

  code {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--color-bg-sunken);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-16) var(--space-6);
    }

    .features {
      grid-template-columns: repeat(3, 1fr);
    }

    h1 {
      font-size: 3rem;
    }
  }
</style>

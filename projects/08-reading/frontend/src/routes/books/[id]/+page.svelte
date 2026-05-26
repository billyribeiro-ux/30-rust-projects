<script lang="ts">
  import { enhance } from '$app/forms';
  import { ArrowLeft, Plus, Trash, Quotes, Timer } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { BookModel } from '$lib/book-model.svelte';
  import type { Highlight, Session } from '$lib/types';
  import { STATUS_LABEL } from '$lib/types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // The BookModel is itself reactive (its fields are $state in the class).
  // We don't double-wrap with another $state; we just keep one instance
  // and sync its fields from `data.book` whenever SvelteKit reruns `load`.
  // svelte-ignore state_referenced_locally
  const book = new BookModel(data.book);
  $effect(() => {
    book.title = data.book.title;
    book.author = data.book.author;
    book.status = data.book.status;
    book.current_page = data.book.current_page;
    book.pages = data.book.pages;
  });

  // $state.raw: large immutable-shaped data we only ever REPLACE (never
  // mutate piecewise). Svelte skips proxy wrapping → faster reads and
  // accidental mutations would throw. The $effect below replaces the
  // whole array whenever `data` changes.
  let highlights = $state.raw<Highlight[]>([]);
  let sessions = $state.raw<Session[]>([]);

  $effect(() => {
    highlights = data.highlights;
    sessions = data.sessions;
  });

  // Update form
  let updateForm: HTMLFormElement | undefined = $state();
  function commitProgress() {
    queueMicrotask(() => updateForm?.requestSubmit());
  }

  // Session form
  let sPages = $state(0);
  let sMins = $state(0);

  // Highlight form
  let hQuote = $state('');
  let hNote = $state('');
  let hPage = $state<number | null>(null);

  // Delete forms
  let deleteSessionForm: HTMLFormElement | undefined = $state();
  let deleteSessionId = $state('');
  function handleDeleteSession(id: string) {
    deleteSessionId = id;
    queueMicrotask(() => deleteSessionForm?.requestSubmit());
  }
  let deleteHighlightForm: HTMLFormElement | undefined = $state();
  let deleteHighlightId = $state('');
  function handleDeleteHighlight(id: string) {
    deleteHighlightId = id;
    queueMicrotask(() => deleteHighlightForm?.requestSubmit());
  }

  const updatePayload = $derived(() =>
    JSON.stringify({
      status: book.status,
      current_page: book.current_page
    })
  );

  function statusButton(s: BookModel['status']) {
    return { selected: book.status === s };
  }
</script>

<svelte:head>
  <title>{book.title} — Reading</title>
</svelte:head>

<main>
  <a href="/" class="back">
    <Icon icon={ArrowLeft} size={16} />
    <span>All books</span>
  </a>

  <article class="hero">
    <div class="cover">
      {#if book.cover_url}
        <img src={book.cover_url} alt="" />
      {:else}
        <div class="cover-fallback" aria-hidden="true">
          {book.title.slice(0, 2).toUpperCase()}
        </div>
      {/if}
    </div>
    <div class="info">
      <h1>{book.title}</h1>
      {#if book.author}<p class="author">by {book.author}</p>{/if}
      {#if book.isbn}<p class="isbn">ISBN: <code>{book.isbn}</code></p>{/if}

      <form
        bind:this={updateForm}
        method="POST"
        action="?/updateBook"
        use:enhance={() => async ({ update }) => update()}
      >
        <input type="hidden" name="payload" value={updatePayload()} />

        <div class="status-row" role="tablist" aria-label="Reading status">
          {#each (['want_to_read', 'reading', 'finished'] as const) as s (s)}
            <button
              type="button"
              role="tab"
              aria-selected={statusButton(s).selected}
              class:active={statusButton(s).selected}
              onclick={() => {
                book.status = s;
                commitProgress();
              }}
            >
              {STATUS_LABEL[s]}
            </button>
          {/each}
        </div>

        {#if book.status === 'reading' && book.pages}
          <label class="progress-input">
            <span>Current page (of {book.pages})</span>
            <input
              type="number"
              min="0"
              max={book.pages}
              bind:value={book.current_page}
              onchange={commitProgress}
            />
          </label>
          <div class="progress-bar" role="progressbar" aria-valuenow={book.progress} aria-valuemin="0" aria-valuemax="100">
            <div class="bar" style="--value: {book.progress}%"></div>
            <p>{book.progress}%</p>
          </div>
        {/if}
      </form>
    </div>
  </article>

  <section class="panel" aria-label="Reading sessions">
    <h2>
      <Icon icon={Timer} size={18} weight="bold" />
      <span>Sessions</span>
    </h2>

    <form
      method="POST"
      action="?/addSession"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') {
          sPages = 0;
          sMins = 0;
        }
        await update();
      }}
      class="session-form"
    >
      <label class="field">
        <span>Pages read</span>
        <input type="number" name="pages_read" min="1" bind:value={sPages} required />
      </label>
      <label class="field">
        <span>Duration (minutes)</span>
        <input type="number" name="duration_minutes" min="1" bind:value={sMins} required />
      </label>
      <button type="submit" class="primary" disabled={sPages <= 0 || sMins <= 0}>
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Log session</span>
      </button>
    </form>

    {#if sessions.length === 0}
      <p class="empty">No sessions yet.</p>
    {:else}
      <ul class="sessions">
        {#each sessions as s (s.id)}
          <li>
            <span class="when">{new Date(s.occurred_at).toLocaleDateString()}</span>
            <span class="amt">{s.pages_read} pages</span>
            <span class="amt">{s.duration_minutes} min</span>
            <button
              type="button"
              class="iconbtn"
              aria-label="Delete session"
              onclick={() => handleDeleteSession(s.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel" aria-label="Highlights">
    <h2>
      <Icon icon={Quotes} size={18} weight="bold" />
      <span>Highlights</span>
    </h2>

    <form
      method="POST"
      action="?/addHighlight"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') {
          hQuote = '';
          hNote = '';
          hPage = null;
        }
        await update();
      }}
      class="highlight-form"
    >
      <label class="field">
        <span>Quote</span>
        <textarea
          name="quote"
          bind:value={hQuote}
          required
          maxlength="2000"
          rows="3"
          placeholder="The passage that stuck with you"
        ></textarea>
      </label>
      <div class="row two">
        <label class="field">
          <span>Note (optional)</span>
          <input type="text" name="note" bind:value={hNote} maxlength="200" />
        </label>
        <label class="field">
          <span>Page (optional)</span>
          <input
            type="number"
            name="page"
            min="0"
            value={hPage ?? ''}
            oninput={(e) => {
              const v = (e.target as HTMLInputElement).value;
              hPage = v === '' ? null : Number(v);
            }}
          />
        </label>
      </div>
      <button type="submit" class="primary" disabled={hQuote.trim().length === 0}>
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Add highlight</span>
      </button>
    </form>

    {#if highlights.length === 0}
      <p class="empty">No highlights yet.</p>
    {:else}
      <ul class="highlights">
        {#each highlights as h (h.id)}
          <li>
            <blockquote>{h.quote}</blockquote>
            {#if h.note}<p class="note">{h.note}</p>{/if}
            <p class="meta">
              {#if h.page !== null}<span>Page {h.page}</span> · {/if}
              <span>{new Date(h.created_at).toLocaleDateString()}</span>
            </p>
            <button
              type="button"
              class="iconbtn"
              aria-label="Delete highlight"
              onclick={() => handleDeleteHighlight(h.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel danger-panel" aria-label="Danger zone">
    <h2>Danger zone</h2>
    <form
      method="POST"
      action="?/removeBook"
      onsubmit={(e) => {
        if (!confirm(`Delete "${book.title}" and all its sessions + highlights?`))
          e.preventDefault();
      }}
    >
      <button type="submit" class="danger">
        <Icon icon={Trash} size={16} weight="bold" />
        <span>Delete this book</span>
      </button>
    </form>
  </section>

  <form
    bind:this={deleteSessionForm}
    method="POST"
    action="?/removeSession"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteSessionId} />
  </form>
  <form
    bind:this={deleteHighlightForm}
    method="POST"
    action="?/removeHighlight"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteHighlightId} />
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
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

  .hero {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: var(--space-4);
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .cover {
    width: 120px;
    aspect-ratio: 2 / 3;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
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
    font-size: var(--text-2xl);
    font-weight: 700;
  }
  .info {
    display: grid;
    gap: var(--space-2);
    align-content: start;
  }
  h1 {
    font-size: var(--text-2xl);
    font-weight: 800;
    line-height: var(--leading-tight);
  }
  .author {
    color: var(--color-fg-muted);
  }
  .isbn {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .isbn code {
    font-family: var(--font-mono);
    font-size: 0.9em;
  }

  .status-row {
    display: inline-flex;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    padding: 2px;
    flex-wrap: wrap;
  }
  .status-row button {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .status-row button.active {
    background: var(--color-bg);
    color: var(--color-fg);
    box-shadow: var(--shadow-sm);
  }

  .progress-input {
    display: grid;
    gap: var(--space-1);
    max-width: 240px;
    margin-top: var(--space-2);
  }
  .progress-input > span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .progress-input input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .progress-bar {
    margin-top: var(--space-2);
  }
  .bar {
    height: 6px;
    background: var(--color-bg-sunken);
    border-radius: 999px;
    overflow: hidden;
    position: relative;
  }
  .bar::after {
    content: '';
    position: absolute;
    inset: 0;
    width: var(--value);
    background: var(--color-accent);
  }
  .progress-bar p {
    margin-top: 2px;
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
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
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .session-form,
  .highlight-form {
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
  .row.two {
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
  .primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sessions {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-2);
  }
  .sessions li {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .when {
    color: var(--color-fg-muted);
  }
  .amt {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .iconbtn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
  }
  .iconbtn:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }

  .highlights {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }
  .highlights li {
    position: relative;
    padding: var(--space-4);
    padding-right: var(--space-10);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--color-accent);
  }
  blockquote {
    font-style: italic;
    color: var(--color-fg);
    line-height: var(--leading-relaxed);
  }
  .note {
    margin-top: var(--space-2);
    color: var(--color-fg);
    font-size: var(--text-sm);
  }
  .highlights .meta {
    margin-top: var(--space-2);
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
  }
  .highlights .iconbtn {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
  }

  .empty {
    color: var(--color-fg-muted);
    padding: var(--space-4);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    text-align: center;
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
    main {
      padding: var(--space-12) var(--space-6);
    }
    .row.two,
    .session-form {
      grid-template-columns: 1fr 1fr;
      align-items: end;
    }
    .session-form .primary {
      grid-column: span 2;
    }
  }
</style>

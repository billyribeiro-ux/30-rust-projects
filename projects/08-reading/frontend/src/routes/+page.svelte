<script lang="ts">
  import { enhance } from '$app/forms';
  import { BookOpen, Plus, MagnifyingGlass, Trash } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import BookCard from '$lib/components/BookCard.svelte';
  import { STATUS_LABEL, type BookStatus } from '$lib/types';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  const books = $derived(data.books);

  // Filter by status — URL is overkill here, we keep it local for the lesson.
  let statusFilter = $state<BookStatus | 'all'>('all');
  const filtered = $derived(
    statusFilter === 'all' ? books : books.filter((b) => b.status === statusFilter)
  );

  // Hidden delete form
  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');
  function handleDelete(id: string) {
    if (!confirm('Delete this book and all its sessions + highlights?')) return;
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }

  // Add-book form (manual)
  let manualTitle = $state('');
  let manualAuthor = $state('');
  let manualStatus = $state<BookStatus>('want_to_read');

  // Lookup form
  let lookupIsbn = $state('');

  const addBookError = $derived(
    form && (form as { kind?: string }).kind === 'addBook' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
  const lookupError = $derived(
    form && (form as { kind?: string }).kind === 'lookupIsbn' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={BookOpen} size={28} weight="duotone" />
      <h1>Reading</h1>
    </div>
    <p class="lead">Track what you read. Highlight what stuck. Remember why.</p>
  </header>

  <!-- STREAMED RETURN: the page paints with `books` immediately, and the
       stats panel below renders a skeleton until the streamed promise
       resolves. SvelteKit handles the chunked transfer-encoding for us. -->
  {#await data.streamed.stats}
    <section class="stats" aria-label="Stats" aria-busy="true">
      <div class="skel"></div>
      <div class="skel"></div>
      <div class="skel"></div>
      <div class="skel"></div>
    </section>
  {:then stats}
    <dl class="stats" aria-label="Stats">
      <div>
        <dt>Books</dt>
        <dd>{stats.total}</dd>
      </div>
      <div>
        <dt>Reading</dt>
        <dd>{stats.reading}</dd>
      </div>
      <div>
        <dt>Finished</dt>
        <dd>{stats.finished}</dd>
      </div>
      <div>
        <dt>Pages read</dt>
        <dd>{stats.pagesRead.toLocaleString()}</dd>
      </div>
    </dl>
  {:catch err}
    <p class="error" role="alert">Stats unavailable: {(err as Error).message}</p>
  {/await}

  <section class="panel" aria-label="Add a book">
    <h2>Add a book</h2>

    <!-- Lookup-by-ISBN form. Failures (Open Library down, invalid ISBN, no
         match) are surfaced via `lookupError` rather than crashing. The
         <svelte:boundary> below is the second layer of defense — it would
         catch a render-time error in this section. -->
    <svelte:boundary onerror={(e) => console.error('add-book section error', e)}>
      <form
        method="POST"
        action="?/lookupIsbn"
        use:enhance={() => async ({ result, update }) => {
          if (result.type === 'success') lookupIsbn = '';
          await update();
        }}
        class="lookup"
      >
        <label class="field">
          <span>Lookup by ISBN</span>
          <input
            type="text"
            name="isbn"
            bind:value={lookupIsbn}
            placeholder="e.g., 978-0-13-468599-1"
            inputmode="numeric"
            autocomplete="off"
          />
        </label>
        <button type="submit" class="primary">
          <Icon icon={MagnifyingGlass} size={16} weight="bold" />
          <span>Look up</span>
        </button>
        {#if lookupError}
          <p class="error" role="alert">{lookupError}</p>
        {/if}
      </form>

      {#snippet failed(err, reset)}
        <div class="error" role="alert">
          <p>The book-lookup section crashed: {(err as Error).message}</p>
          <button type="button" onclick={reset}>Try again</button>
        </div>
      {/snippet}
    </svelte:boundary>

    <div class="or"><span>or</span></div>

    <form
      method="POST"
      action="?/addBook"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') {
          manualTitle = '';
          manualAuthor = '';
        }
        await update();
      }}
      class="manual"
    >
      <div class="row two">
        <label class="field">
          <span>Title</span>
          <input
            type="text"
            name="title"
            bind:value={manualTitle}
            required
            maxlength="200"
          />
        </label>
        <label class="field">
          <span>Author</span>
          <input type="text" name="author" bind:value={manualAuthor} maxlength="200" />
        </label>
      </div>
      <label class="field">
        <span>Status</span>
        <select bind:value={manualStatus} name="status">
          <option value="want_to_read">Want to read</option>
          <option value="reading">Reading</option>
          <option value="finished">Finished</option>
        </select>
      </label>
      <button type="submit" class="primary">
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Add manually</span>
      </button>
      {#if addBookError}
        <p class="error" role="alert">{addBookError}</p>
      {/if}
    </form>
  </section>

  <section class="panel" aria-label="Books">
    <header class="library-header">
      <h2>Library</h2>
      <div class="filters" role="tablist" aria-label="Status filter">
        {#each ['all', 'reading', 'want_to_read', 'finished'] as f, i (i)}
          <button
            type="button"
            role="tab"
            aria-selected={statusFilter === f}
            class:active={statusFilter === f}
            onclick={() => (statusFilter = f as typeof statusFilter)}
          >
            {f === 'all' ? 'All' : STATUS_LABEL[f as BookStatus]}
          </button>
        {/each}
      </div>
    </header>

    {#if filtered.length === 0}
      <p class="empty">
        {#if statusFilter === 'all'}
          No books yet. Add your first above.
        {:else}
          No books with status "{STATUS_LABEL[statusFilter as BookStatus] ?? statusFilter}".
        {/if}
      </p>
    {:else}
      <ul class="grid">
        {#each filtered as book (book.id)}
          <li>
            <BookCard {book} />
            <button
              type="button"
              class="del"
              aria-label={`Delete ${book.title}`}
              onclick={() => handleDelete(book.id)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <form
    bind:this={deleteForm}
    method="POST"
    action="?/removeBook"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteId} />
  </form>
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

  .stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .stats > div {
    display: grid;
    gap: 2px;
  }
  .stats dt {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stats dd {
    font-size: var(--text-xl);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .skel {
    height: 32px;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0% { opacity: 0.5; }
    50% { opacity: 1; }
    100% { opacity: 0.5; }
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

  .lookup,
  .manual {
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
  .field select {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .row.two {
    display: grid;
    gap: var(--space-3);
  }

  .or {
    text-align: center;
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    position: relative;
  }
  .or::before,
  .or::after {
    content: '';
    position: absolute;
    top: 50%;
    width: 30%;
    height: 1px;
    background: var(--color-border);
  }
  .or::before {
    left: 0;
  }
  .or::after {
    right: 0;
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

  .library-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .filters {
    display: inline-flex;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    padding: 2px;
  }
  .filters button {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .filters button.active {
    background: var(--color-bg);
    color: var(--color-fg);
    box-shadow: var(--shadow-sm);
  }

  .grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }
  .grid li {
    position: relative;
  }
  .del {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-muted);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out);
  }
  .grid li:hover .del,
  .del:focus-visible {
    opacity: 1;
  }
  .del:hover {
    color: hsl(0 72% 42%);
  }

  .empty {
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }
    .stats {
      grid-template-columns: repeat(4, 1fr);
    }
    .row.two {
      grid-template-columns: 1fr 1fr;
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

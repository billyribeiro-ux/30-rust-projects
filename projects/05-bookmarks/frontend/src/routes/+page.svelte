<script lang="ts">
  import { enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { MagnifyingGlass, Plus, X, BookmarkSimple } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import BookmarkCard from '$lib/components/BookmarkCard.svelte';
  import TagSidebar from '$lib/components/TagSidebar.svelte';
  import { clickOutside } from '$lib/actions';
  import { debounce } from '$lib/debounce';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  const bookmarks = $derived(data.bookmarks);
  const tags = $derived(data.tags);
  const activeTag = $derived(data.tag);

  // Search input value — local typing buffer. The URL is the source of truth
  // (data.q), and we sync changes FROM the URL to this buffer via $effect
  // (handles browser back/forward). Changes FROM the buffer to the URL go
  // through `commitSearch` debounce → goto.
  let searchInput = $state('');
  $effect(() => {
    searchInput = data.q;
  });
  let addOpen = $state(false);

  let createForm: HTMLFormElement | undefined = $state();
  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');

  // ---- URL-as-state ----
  // The URL is the source of truth for `q` and `tag`. Editing them goes
  // through `applyFilters` → goto, which re-runs `load` and updates `data`.
  // SvelteKit auto-handles back/forward navigation for free.
  function applyFilters(next: { q?: string; tag?: string }) {
    const url = new URL(page.url);
    const q = (next.q ?? activeQ()).trim();
    const tag = (next.tag ?? activeTag).trim();
    if (q) url.searchParams.set('q', q);
    else url.searchParams.delete('q');
    if (tag) url.searchParams.set('tag', tag);
    else url.searchParams.delete('tag');
    void goto(`${url.pathname}${url.search}`, { keepFocus: true, noScroll: true, replaceState: false });
  }

  function activeQ(): string {
    return page.url.searchParams.get('q') ?? '';
  }

  // Debounced URL update on keystroke. The local input state updates instantly
  // (responsive UI), but only commits to the URL (and triggers `load`) after
  // 200ms of no further typing.
  const commitSearch = debounce((value: string) => {
    applyFilters({ q: value });
  }, 200);

  function onSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchInput = value;
    commitSearch(value);
  }

  function clearSearch() {
    commitSearch.cancel();
    searchInput = '';
    applyFilters({ q: '' });
  }

  function selectTag(tag: string) {
    applyFilters({ tag });
  }

  function handleDelete(id: string) {
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={BookmarkSimple} size={28} weight="duotone" />
      <h1>Bookmarks</h1>
    </div>

    <div class="search-wrap" role="search">
      <span class="search-icon" aria-hidden="true">
        <Icon icon={MagnifyingGlass} size={16} />
      </span>
      <input
        type="search"
        value={searchInput}
        oninput={onSearchInput}
        placeholder="Search title, description, URL…"
        aria-label="Search bookmarks"
      />
      {#if searchInput}
        <button type="button" class="search-clear" onclick={clearSearch} aria-label="Clear search">
          <Icon icon={X} size={14} weight="bold" />
        </button>
      {/if}
    </div>

    <button
      type="button"
      class="primary"
      onclick={() => (addOpen = !addOpen)}
      aria-expanded={addOpen}
    >
      <Icon icon={Plus} size={18} weight="bold" />
      <span>Add bookmark</span>
    </button>
  </header>

  {#if addOpen}
    <section
      class="add-panel"
      aria-label="Add bookmark"
      use:clickOutside={() => (addOpen = false)}
    >
      <form
        bind:this={createForm}
        method="POST"
        action="?/create"
        use:enhance={() => async ({ result, update }) => {
          if (result.type === 'success') addOpen = false;
          await update();
        }}
      >
        <label class="field">
          <span>URL</span>
          <input
            type="url"
            name="url"
            required
            placeholder="https://example.com/article"
            value={(form?.url as string | undefined) ?? ''}
          />
        </label>

        <label class="field">
          <span>Title</span>
          <input
            type="text"
            name="title"
            required
            maxlength="200"
            placeholder="A short, descriptive title"
            value={(form?.title as string | undefined) ?? ''}
          />
        </label>

        <label class="field">
          <span>Description</span>
          <textarea
            name="description"
            maxlength="1000"
            placeholder="Why is it worth remembering?"
            rows="2">{(form?.description as string | undefined) ?? ''}</textarea>
        </label>

        <label class="field">
          <span>Tags <em>(comma- or space-separated)</em></span>
          <input
            type="text"
            name="tags"
            placeholder="rust axum tokio"
            value={(form?.tagsRaw as string | undefined) ?? ''}
          />
        </label>

        {#if form && 'error' in form && form.error}
          <p class="error" role="alert">{form.error}</p>
        {/if}

        <div class="actions">
          <button type="button" class="ghost" onclick={() => (addOpen = false)}>Cancel</button>
          <button type="submit" class="primary">Save bookmark</button>
        </div>
      </form>
    </section>
  {/if}

  <div class="grid">
    <aside class="aside">
      <TagSidebar {tags} {activeTag} onSelect={selectTag} />
    </aside>

    <section class="results" aria-live="polite" aria-busy="false">
      {#if bookmarks.length === 0}
        <p class="empty">
          {#if activeTag || activeQ()}
            No bookmarks match your filters.
            <button type="button" class="link" onclick={() => applyFilters({ q: '', tag: '' })}>
              Clear all
            </button>
          {:else}
            No bookmarks yet. Click <strong>Add bookmark</strong> above to save your first one.
          {/if}
        </p>
      {:else}
        <p class="count" aria-live="polite">
          {bookmarks.length} {bookmarks.length === 1 ? 'result' : 'results'}
        </p>
        <ul class="cards">
          {#each bookmarks as bookmark (bookmark.id)}
            <li>
              <BookmarkCard
                {bookmark}
                {activeTag}
                onDelete={handleDelete}
                onTagClick={selectTag}
              />
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>

  <form
    bind:this={deleteForm}
    method="POST"
    action="?/remove"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteId} />
  </form>
</main>

<style>
  main {
    max-width: var(--container-xl);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }

  header {
    display: grid;
    gap: var(--space-3);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent);
  }

  h1 {
    font-size: var(--text-2xl);
    font-weight: 800;
    color: var(--color-fg);
  }

  .search-wrap {
    position: relative;
  }

  .search-wrap input {
    width: 100%;
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-10);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
  }

  .search-wrap input:focus-visible {
    border-color: var(--color-accent);
    outline-offset: 0;
  }

  .search-icon {
    position: absolute;
    top: 50%;
    left: var(--space-3);
    transform: translateY(-50%);
    color: var(--color-fg-muted);
    pointer-events: none;
  }

  .search-clear {
    position: absolute;
    top: 50%;
    right: var(--space-2);
    transform: translateY(-50%);
    width: 28px;
    height: 28px;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .search-clear:hover {
    color: var(--color-fg);
    background: var(--color-bg-sunken);
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
  }

  .primary:hover {
    background: var(--color-accent-hover);
  }

  .ghost {
    padding: var(--space-3) var(--space-4);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    background: var(--color-bg);
    border-radius: var(--radius-md);
    font-weight: 500;
  }

  .add-panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .add-panel form {
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

  .field > span em {
    font-style: normal;
    color: var(--color-fg-subtle);
  }

  .field input,
  .field textarea {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    resize: vertical;
  }

  .add-panel .actions {
    display: flex;
    gap: var(--space-3);
    justify-content: flex-end;
  }

  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-6);
  }

  .aside {
    order: 2;
  }

  .results {
    order: 1;
    display: grid;
    gap: var(--space-3);
  }

  .count {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }

  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }

  .empty {
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }

  .link {
    color: var(--color-accent);
    text-decoration: underline;
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }

    header {
      grid-template-columns: auto 1fr auto;
      align-items: center;
    }

    .grid {
      grid-template-columns: 220px 1fr;
    }

    .aside {
      order: 0;
    }

    .results {
      order: 0;
    }

    .cards {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (min-width: 1024px) {
    .cards {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>

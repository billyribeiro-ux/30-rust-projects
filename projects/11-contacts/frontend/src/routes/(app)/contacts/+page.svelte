<script lang="ts">
  import { enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const contacts = $derived(data.contacts);

  let q = $state('');
  $effect(() => {
    q = data.q;
  });

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');
  function handleDelete(id: string, name: string) {
    if (!confirm(`Delete ${name}?`)) return;
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  function onSearch(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    q = value;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      const url = new URL(page.url);
      if (value) url.searchParams.set('q', value);
      else url.searchParams.delete('q');
      void goto(`${url.pathname}${url.search}`, { keepFocus: true, noScroll: true });
    }, 200);
  }
</script>

<svelte:head><title>Contacts — Contacts</title></svelte:head>

<main>
  <header>
    <h1>Contacts</h1>
    <a class="primary" href="/contacts/new">Add contact</a>
  </header>

  <div class="search" role="search">
    <input
      type="search"
      value={q}
      oninput={onSearch}
      placeholder="Search name, email, company, notes…"
      aria-label="Search contacts"
    />
  </div>

  {#if contacts.length === 0}
    <p class="empty">
      {#if data.q || data.tag}
        No contacts match. <a href="/contacts">Clear filters</a>
      {:else}
        No contacts yet. <a href="/contacts/new">Add your first</a>.
      {/if}
    </p>
  {:else}
    <p class="count">{contacts.length} {contacts.length === 1 ? 'contact' : 'contacts'}</p>
    <ul class="list">
      {#each contacts as c (c.id)}
        <li>
          <div class="meta">
            <a class="name" href="/contacts/{c.id}">{c.name}</a>
            {#if c.company}<p class="company">{c.company}</p>{/if}
            {#if c.email}<p class="email">{c.email}</p>{/if}
            {#if c.tags.length > 0}
              <ul class="tags" aria-label="Tags">
                {#each c.tags as t (t)}
                  <li><a href="/contacts?tag={encodeURIComponent(t)}">{t}</a></li>
                {/each}
              </ul>
            {/if}
          </div>
          <button
            type="button"
            class="del"
            aria-label={`Delete ${c.name}`}
            onclick={() => handleDelete(c.id, c.name)}
          >Delete</button>
        </li>
      {/each}
    </ul>
  {/if}

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
  main { max-width: var(--container-lg); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-4); }
  header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--space-3); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .primary:hover { background: var(--color-accent-hover); }
  .search input {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .count { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .list { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .list li {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .name { font-weight: 600; }
  .company, .email { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .tags { list-style: none; padding: 0; margin: var(--space-2) 0 0; display: flex; flex-wrap: wrap; gap: var(--space-1); }
  .tags li a { font-size: var(--text-xs); padding: 2px var(--space-2); background: var(--color-bg-sunken); border-radius: var(--radius-full); color: var(--color-fg-muted); }
  .del { color: var(--color-fg-muted); font-size: var(--text-xs); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); }
  .del:hover { color: hsl(0 72% 42%); background: var(--color-bg-sunken); }
  .empty { padding: var(--space-12) var(--space-4); text-align: center; color: var(--color-fg-muted); background: var(--color-bg-sunken); border-radius: var(--radius-md); }
</style>

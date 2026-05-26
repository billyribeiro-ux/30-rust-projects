<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, Trash, ArrowLeft } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { MUSCLE_GROUPS, type MuscleGroup } from '$lib/types';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  let name = $state('');
  let muscle = $state<MuscleGroup>('chest');

  // Local search (FTS5 lives on the server — we exercise it on the
  // workout-new page where it really matters for autocomplete).
  let search = $state('');
  const filtered = $derived(
    search.trim()
      ? data.exercises.filter((e) =>
          e.name.toLowerCase().includes(search.trim().toLowerCase())
        )
      : data.exercises
  );

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');
  function handleDelete(id: string, exerciseName: string) {
    if (!confirm(`Delete "${exerciseName}"?`)) return;
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }

  const addError = $derived(
    form && (form as { kind?: string }).kind === 'add' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
  const removeError = $derived(
    form && (form as { kind?: string }).kind === 'remove' && 'error' in form
      ? (form as { error?: string }).error
      : ''
  );
</script>

<main>
  <p class="back"><a href="/"><Icon icon={ArrowLeft} size={14} /> Back to dashboard</a></p>

  <header>
    <h1>Exercises</h1>
    <p class="lead">The catalog the autocomplete searches.</p>
  </header>

  <section class="panel" aria-label="Add an exercise">
    <h2>Add an exercise</h2>
    <form
      method="POST"
      action="?/add"
      use:enhance={() => async ({ result, update }) => {
        if (result.type === 'success') name = '';
        await update();
      }}
      class="add"
    >
      <label class="field">
        <span>Name</span>
        <input type="text" name="name" bind:value={name} maxlength="80" required />
      </label>
      <label class="field">
        <span>Muscle group</span>
        <select name="muscle_group" bind:value={muscle}>
          {#each MUSCLE_GROUPS as g (g)}
            <option value={g}>{g}</option>
          {/each}
        </select>
      </label>
      <button type="submit" class="primary">
        <Icon icon={Plus} size={16} weight="bold" />
        <span>Add</span>
      </button>
      {#if addError}<p class="error" role="alert">{addError}</p>{/if}
    </form>
  </section>

  <section class="panel" aria-label="Exercise list">
    <header class="panel-header">
      <h2>Catalog ({data.exercises.length})</h2>
      <label class="search">
        <span class="sr-only">Search</span>
        <input
          type="search"
          bind:value={search}
          placeholder="Search exercises"
          aria-label="Search exercises"
        />
      </label>
    </header>

    {#if removeError}<p class="error" role="alert">{removeError}</p>{/if}

    {#if filtered.length === 0}
      <p class="empty">No exercises match.</p>
    {:else}
      <ul class="list">
        {#each filtered as ex (ex.id)}
          <li>
            <div>
              <p class="name">{ex.name}</p>
              <p class="muted">{ex.muscle_group}</p>
            </div>
            <button
              type="button"
              class="del"
              aria-label={`Delete ${ex.name}`}
              onclick={() => handleDelete(ex.id, ex.name)}
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
    action="?/remove"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteId} />
  </form>
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }
  .back a {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  header { display: grid; gap: var(--space-1); }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }

  .panel {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  h2 { font-size: var(--text-lg); font-weight: 700; }

  .add { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }
  .field input,
  .field select,
  .search input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .search input { min-width: 200px; }
  .sr-only {
    position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0;
  }

  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    justify-self: start;
  }
  .primary:hover { background: var(--color-accent-hover); }

  .list { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .name { font-weight: 600; }
  .muted { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .del {
    width: 32px; height: 32px;
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
  }
  .del:hover { color: var(--color-danger); background: var(--color-bg-sunken); }

  .error {
    padding: var(--space-2) var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .empty {
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }
</style>

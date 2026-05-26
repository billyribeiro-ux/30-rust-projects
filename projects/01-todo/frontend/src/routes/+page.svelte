<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, ListChecks } from 'phosphor-svelte';
  import TodoItem from '$lib/components/TodoItem.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  const todos = $derived(data.todos);
  const remaining = $derived(todos.filter((t) => !t.done).length);
  const total = $derived(todos.length);

  let title = $state('');
  let submitting = $state(false);

  let toggleForm: HTMLFormElement | undefined = $state();
  let toggleId = $state('');
  let toggleDone = $state(false);

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');

  function handleToggle(id: string, done: boolean) {
    toggleId = id;
    toggleDone = done;
    queueMicrotask(() => toggleForm?.requestSubmit());
  }

  function handleDelete(id: string) {
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={ListChecks} size={28} weight="duotone" />
      <h1>Today</h1>
    </div>
    <p class="meta" aria-live="polite">
      {remaining} of {total} remaining
    </p>
  </header>

  <form
    method="POST"
    action="?/create"
    use:enhance={() => {
      submitting = true;
      return async ({ result, update }) => {
        submitting = false;
        if (result.type === 'success') {
          title = '';
        }
        await update();
      };
    }}
    class="add-form"
  >
    <input
      type="text"
      name="title"
      bind:value={title}
      placeholder="What needs to get done?"
      autocomplete="off"
      maxlength="200"
      required
      aria-label="New task"
    />
    <button type="submit" class="primary" disabled={submitting || title.trim().length === 0}>
      <Icon icon={Plus} size={18} weight="bold" />
      <span>Add</span>
    </button>
  </form>

  <form
    bind:this={toggleForm}
    method="POST"
    action="?/toggle"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={toggleId} />
    <input type="hidden" name="done" value={String(toggleDone)} />
  </form>

  <form
    bind:this={deleteForm}
    method="POST"
    action="?/remove"
    use:enhance={() => async ({ update }) => update()}
    hidden
  >
    <input type="hidden" name="id" value={deleteId} />
  </form>

  {#if form && 'error' in form && form.error}
    <p class="error" role="alert">{form.error}</p>
  {/if}

  {#if todos.length === 0}
    <p class="empty">No tasks yet — add your first above.</p>
  {:else}
    <ul class="list">
      {#each todos as todo (todo.id)}
        <TodoItem {todo} onToggle={handleToggle} onDelete={handleDelete} />
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    max-width: 640px;
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-5);
  }

  header {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--color-accent);
  }

  h1 {
    color: var(--color-fg);
    font-size: var(--text-3xl);
    line-height: var(--leading-tight);
    font-weight: 700;
  }

  .meta {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }

  .add-form {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-2);
  }

  .add-form input {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    transition: border-color var(--duration-fast) var(--ease-out);
  }

  .add-form input:focus-visible {
    border-color: var(--color-accent);
    outline-offset: 0;
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
    transition:
      background var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out);
  }

  .primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }

  .primary:active:not(:disabled) {
    transform: translateY(1px);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error {
    padding: var(--space-3) var(--space-4);
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

  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-2);
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
      gap: var(--space-6);
    }

    h1 {
      font-size: var(--text-4xl);
    }
  }
</style>

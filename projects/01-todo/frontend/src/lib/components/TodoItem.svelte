<script lang="ts">
  import { Check, Trash } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import type { Todo } from '$lib/types';

  type Props = {
    todo: Todo;
    onToggle: (id: string, done: boolean) => void;
    onDelete: (id: string) => void;
  };

  let { todo, onToggle, onDelete }: Props = $props();

  let pending = $state(false);

  async function toggle() {
    pending = true;
    try {
      onToggle(todo.id, !todo.done);
    } finally {
      pending = false;
    }
  }

  async function remove() {
    pending = true;
    try {
      onDelete(todo.id);
    } finally {
      pending = false;
    }
  }
</script>

<li class="todo" class:done={todo.done} class:pending>
  <button
    type="button"
    class="checkbox"
    aria-pressed={todo.done}
    aria-label={todo.done ? `Mark "${todo.title}" as not done` : `Mark "${todo.title}" as done`}
    onclick={toggle}
    disabled={pending}
  >
    {#if todo.done}
      <Icon icon={Check} size={16} weight="bold" />
    {/if}
  </button>

  <span class="title">{todo.title}</span>

  <button
    type="button"
    class="remove"
    aria-label={`Delete "${todo.title}"`}
    onclick={remove}
    disabled={pending}
  >
    <Icon icon={Trash} size={18} />
  </button>
</li>

<style>
  .todo {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out),
      opacity var(--duration-fast) var(--ease-out);
  }

  .todo:hover {
    border-color: var(--color-border-strong);
  }

  .todo.done .title {
    color: var(--color-fg-muted);
    text-decoration: line-through;
    text-decoration-thickness: 1px;
  }

  .todo.pending {
    opacity: 0.6;
    pointer-events: none;
  }

  .checkbox {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 2px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-accent-fg);
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out);
  }

  .checkbox[aria-pressed='true'] {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }

  .title {
    font-size: var(--text-base);
    word-break: break-word;
  }

  .remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    color: var(--color-fg-subtle);
    border-radius: var(--radius-sm);
    transition: color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out);
  }

  .remove:hover {
    color: var(--color-danger);
    background: var(--color-bg-sunken);
  }

  @media (min-width: 768px) {
    .todo {
      padding: var(--space-4) var(--space-5);
    }
  }
</style>

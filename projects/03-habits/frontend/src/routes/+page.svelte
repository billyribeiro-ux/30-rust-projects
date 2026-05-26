<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, Flame } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import HabitRow from '$lib/components/HabitRow.svelte';
  import { ALLOWED_COLORS } from '$lib/types';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  const habits = $derived(data.habits);

  let name = $state('');
  let color = $state<string>(ALLOWED_COLORS[0]);
  let submitting = $state(false);

  let toggleForm: HTMLFormElement | undefined = $state();
  let toggleId = $state('');
  let toggleDate = $state('');

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');

  // Aggregate stats across all habits
  const totals = $derived({
    habits: habits.length,
    activeStreaks: habits.filter((h) => h.streak.current > 0).length,
    longest: habits.reduce((m, h) => Math.max(m, h.streak.longest), 0)
  });

  function handleToggle(id: string, date: string) {
    toggleId = id;
    toggleDate = date;
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
      <Icon icon={Flame} size={28} weight="fill" color="var(--color-danger)" />
      <h1>Habits</h1>
    </div>
    <dl class="totals" aria-label="Overall totals">
      <div>
        <dt>Habits</dt>
        <dd>{totals.habits}</dd>
      </div>
      <div>
        <dt>Active streaks</dt>
        <dd>{totals.activeStreaks}</dd>
      </div>
      <div>
        <dt>Longest</dt>
        <dd>{totals.longest}</dd>
      </div>
    </dl>
  </header>

  <form
    method="POST"
    action="?/create"
    use:enhance={() => {
      submitting = true;
      return async ({ result, update }) => {
        submitting = false;
        if (result.type === 'success') name = '';
        await update();
      };
    }}
    class="add-form"
  >
    <input
      type="text"
      name="name"
      bind:value={name}
      placeholder="A new habit"
      autocomplete="off"
      maxlength="80"
      required
      aria-label="New habit name"
    />

    <fieldset class="palette" aria-label="Habit color">
      <legend class="sr-only">Habit color</legend>
      {#each ALLOWED_COLORS as c (c)}
        <label class="swatch" style="--swatch: {c}">
          <input type="radio" name="color" value={c} bind:group={color} />
          <span aria-hidden="true"></span>
          <span class="sr-only">{c}</span>
        </label>
      {/each}
    </fieldset>

    <button type="submit" class="primary" disabled={submitting || name.trim().length === 0}>
      <Icon icon={Plus} size={18} weight="bold" />
      <span>Add habit</span>
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
    <input type="hidden" name="date" value={toggleDate} />
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

  {#if habits.length === 0}
    <p class="empty">No habits yet. Add your first above and start a streak today.</p>
  {:else}
    <section class="list" aria-label="Your habits">
      {#each habits as habit (habit.id)}
        <HabitRow {habit} onToggle={handleToggle} onDelete={handleDelete} />
      {/each}
    </section>
  {/if}
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
    gap: var(--space-4);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
    line-height: var(--leading-tight);
  }

  .totals {
    display: flex;
    gap: var(--space-6);
    flex-wrap: wrap;
  }

  .totals > div {
    display: grid;
    gap: 2px;
  }

  .totals dt {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .totals dd {
    font-size: var(--text-2xl);
    font-weight: 700;
    line-height: 1;
  }

  .add-form {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .add-form input[type='text'] {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
  }

  .palette {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    border: 0;
    padding: 0;
  }

  .swatch {
    position: relative;
    width: 28px;
    height: 28px;
    cursor: pointer;
    display: inline-block;
  }

  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }

  .swatch span[aria-hidden] {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 999px;
    background: var(--swatch);
    border: 2px solid transparent;
    transition: transform var(--duration-fast) var(--ease-out);
  }

  .swatch input:checked + span[aria-hidden] {
    border-color: var(--color-fg);
    transform: scale(1.1);
  }

  .swatch input:focus-visible + span[aria-hidden] {
    outline: 3px solid var(--color-focus);
    outline-offset: 2px;
  }

  .primary {
    justify-self: end;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
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
    padding: var(--space-12) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
  }

  .list {
    display: grid;
    gap: var(--space-4);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (min-width: 768px) {
    main {
      padding: var(--space-12) var(--space-6);
    }

    header {
      grid-template-columns: 1fr auto;
      align-items: end;
    }

    h1 {
      font-size: var(--text-4xl);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .primary,
    .swatch span[aria-hidden] {
      transition: none;
    }
  }
</style>

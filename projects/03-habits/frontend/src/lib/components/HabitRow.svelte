<script lang="ts">
  import { Trash, Fire } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import CalendarGrid from './CalendarGrid.svelte';
  import type { Habit } from '$lib/types';

  type Props = {
    habit: Habit;
    onToggle: (id: string, date: string) => void;
    onDelete: (id: string) => void;
  };

  let { habit, onToggle, onDelete }: Props = $props();

  const completionsSet = $derived(new Set(habit.completions));
  const streak = $derived(habit.streak);
</script>

<article class="row" style="--habit-color: {habit.color}">
  <header>
    <div class="title">
      <span class="dot" aria-hidden="true"></span>
      <h2>{habit.name}</h2>
    </div>

    <dl class="stats" aria-label={`${habit.name} streak`}>
      <div>
        <dt>
          <Icon icon={Fire} size={16} weight="fill" />
          <span>Current</span>
        </dt>
        <dd>{streak.current}</dd>
      </div>
      <div>
        <dt>Longest</dt>
        <dd>{streak.longest}</dd>
      </div>
      <div>
        <dt>Total</dt>
        <dd>{streak.total}</dd>
      </div>
    </dl>

    <button
      type="button"
      class="delete"
      aria-label={`Delete habit "${habit.name}"`}
      onclick={() => {
        if (confirm(`Delete "${habit.name}"? All completions will be removed.`)) {
          onDelete(habit.id);
        }
      }}
    >
      <Icon icon={Trash} size={16} />
    </button>
  </header>

  <CalendarGrid
    habitId={habit.id}
    habitName={habit.name}
    color={habit.color}
    completions={completionsSet}
    onToggle={(date) => onToggle(habit.id, date)}
  />
</article>

<style>
  .row {
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-4);
  }

  header {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: var(--space-3);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    grid-column: 1 / -1;
  }

  .dot {
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: var(--habit-color);
  }

  h2 {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .stats {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .stats > div {
    display: grid;
    gap: 2px;
  }

  .stats dt {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .stats dd {
    font-size: var(--text-xl);
    font-weight: 700;
    line-height: 1;
    color: var(--habit-color);
  }

  .delete {
    color: var(--color-fg-muted);
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    justify-self: end;
  }

  .delete:hover {
    color: hsl(0 72% 42%);
    background: var(--color-bg-sunken);
  }

  @media (min-width: 768px) {
    header {
      grid-template-columns: 1fr auto auto;
    }
    .title {
      grid-column: 1;
    }
  }
</style>

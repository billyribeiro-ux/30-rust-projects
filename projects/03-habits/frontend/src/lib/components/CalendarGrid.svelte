<script lang="ts">
  type Props = {
    habitId: string;
    habitName: string;
    color: string;
    completions: Set<string>;
    onToggle: (date: string) => void;
  };

  let { habitId, habitName, color, completions, onToggle }: Props = $props();

  // Build a 7x7 grid of the last 49 days, oldest column first.
  // Rows are weekdays (Mon..Sun), columns are weeks (oldest..newest).
  const cells = $derived(buildCells());

  function buildCells(): { iso: string; label: string; isToday: boolean }[] {
    const out: { iso: string; label: string; isToday: boolean }[] = [];
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const todayIso = isoDate(today);
    for (let i = 48; i >= 0; i--) {
      const d = new Date(today);
      d.setDate(today.getDate() - i);
      const iso = isoDate(d);
      out.push({
        iso,
        label: d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }),
        isToday: iso === todayIso
      });
    }
    return out;
  }

  function isoDate(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  // Cells are laid out as columns of 7 (each column = one week, oldest first).
  // Index in `cells` runs 0..48; col = floor(i/7), row = i%7.
  const COLS = 7;
  const ROWS = 7;

  function handleKey(e: KeyboardEvent, index: number) {
    let next = index;
    switch (e.key) {
      case 'ArrowRight':
        next = Math.min(index + ROWS, cells.length - 1);
        break;
      case 'ArrowLeft':
        next = Math.max(index - ROWS, 0);
        break;
      case 'ArrowDown':
        next = Math.min(index + 1, cells.length - 1);
        break;
      case 'ArrowUp':
        next = Math.max(index - 1, 0);
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = cells.length - 1;
        break;
      default:
        return;
    }
    e.preventDefault();
    const btn = document.querySelector<HTMLButtonElement>(
      `[data-habit="${habitId}"][data-index="${next}"]`
    );
    btn?.focus();
  }
</script>

<div
  class="grid"
  role="group"
  aria-label={`${habitName} — 7 weeks of completions`}
  style="--habit-color: {color}"
>
  {#each cells as cell, i (cell.iso)}
    {@const completed = completions.has(cell.iso)}
    <button
      type="button"
      class="cell"
      class:completed
      class:today={cell.isToday}
      aria-pressed={completed}
      aria-label={`${completed ? 'Unmark' : 'Mark'} ${habitName} for ${cell.label}`}
      data-habit={habitId}
      data-index={i}
      tabindex={i === cells.length - 1 ? 0 : -1}
      onclick={() => onToggle(cell.iso)}
      onkeydown={(e) => handleKey(e, i)}
    ></button>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(7, 1fr);
    grid-auto-flow: column;
    gap: 4px;
    width: fit-content;
  }

  .cell {
    width: clamp(18px, 4vw, 26px);
    height: clamp(18px, 4vw, 26px);
    border-radius: var(--radius-sm);
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    transition: transform var(--duration-fast) var(--ease-out);
  }

  .cell:hover {
    border-color: var(--habit-color);
  }

  .cell.completed {
    background: var(--habit-color);
    border-color: var(--habit-color);
  }

  .cell.today {
    outline: 2px solid var(--color-fg);
    outline-offset: 1px;
  }

  /* Keyboard focus is distinct from the today outline */
  .cell:focus-visible {
    outline: 3px solid var(--color-focus);
    outline-offset: 2px;
  }

  /* Honor reduced-motion */
  @media (prefers-reduced-motion: reduce) {
    .cell {
      transition: none;
    }
  }
</style>

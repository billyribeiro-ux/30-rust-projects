<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, Trash, ArrowLeft, MagnifyingGlass } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { Exercise } from '$lib/types';
  import { parseWeightKg } from '$lib/weight';
  import { exercisesApi } from '$lib/api';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  type DraftSet = {
    exercise: Exercise | null;
    weightKg: string;
    reps: string;
    rir: string;
  };

  let workoutName = $state('');
  let drafts = $state<DraftSet[]>([
    { exercise: null, weightKg: '', reps: '', rir: '0' }
  ]);

  // Autocomplete state — one search popover per row. We track which row
  // owns the popover via an index, and debounce the FTS5 call.
  let activeRow = $state<number | null>(null);
  let query = $state('');
  let suggestions = $state<Exercise[]>(data.exercises.slice(0, 8));
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Debounced FTS5 search. We capture `query` in the closure; cancel
    // any pending search on each keystroke so only the freshest fires.
    const q = query;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(async () => {
      if (!q.trim()) {
        suggestions = data.exercises.slice(0, 8);
        return;
      }
      try {
        suggestions = await exercisesApi.list(fetch, q);
      } catch {
        suggestions = [];
      }
    }, 150);
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  function openPicker(i: number) {
    activeRow = i;
    query = '';
    suggestions = data.exercises.slice(0, 8);
  }
  function pickExercise(ex: Exercise) {
    if (activeRow === null) return;
    const current = drafts[activeRow];
    if (current) {
      drafts[activeRow] = { ...current, exercise: ex };
    }
    activeRow = null;
  }
  function addRow() {
    drafts.push({ exercise: null, weightKg: '', reps: '', rir: '0' });
  }
  function removeRow(i: number) {
    drafts.splice(i, 1);
    if (drafts.length === 0) addRow();
  }

  // Derived: count of valid rows (a row is valid when exercise + weight +
  // reps parse OK). Disable submit until at least one row is valid.
  const validRowCount = $derived(
    drafts.filter(
      (d) => d.exercise && parseWeightKg(d.weightKg) !== null && Number(d.reps) > 0
    ).length
  );

  // Derived: serialized payload for the hidden input.
  const setsPayload = $derived(
    JSON.stringify(
      drafts
        .map((d) => {
          if (!d.exercise) return null;
          const w = parseWeightKg(d.weightKg);
          const r = Number(d.reps);
          const rir = Number(d.rir);
          if (w === null || !Number.isFinite(r) || r <= 0) return null;
          return {
            exercise_id: d.exercise.id,
            weight_minor: w,
            reps: r,
            rir: Number.isFinite(rir) ? Math.max(0, Math.min(10, rir)) : 0
          };
        })
        .filter((s) => s !== null)
    )
  );

  const submitError = $derived(form && 'error' in form ? (form as { error: string }).error : '');
</script>

<main>
  <p class="back"><a href="/"><Icon icon={ArrowLeft} size={14} /> Back to dashboard</a></p>

  <header>
    <h1>New workout</h1>
    <p class="lead">Log sets, see PR badges on the detail page.</p>
  </header>

  {#if data.exercises.length === 0}
    <p class="empty">
      You need exercises before you can log sets. <a href="/exercises">Add some first</a>.
    </p>
  {:else}
    <form
      method="POST"
      action="?/create"
      use:enhance={() => async ({ update }) => update()}
      class="form"
    >
      <label class="field">
        <span>Name (optional)</span>
        <input
          type="text"
          name="name"
          bind:value={workoutName}
          maxlength="200"
          placeholder="e.g. Push day"
        />
      </label>

      <h2>Sets</h2>
      <ol class="sets">
        {#each drafts as draft, i (i)}
          <li class="set-row">
            <div class="set-cell exercise-cell">
              {#if draft.exercise}
                <button type="button" class="picker chosen" onclick={() => openPicker(i)}>
                  <span class="ex-name">{draft.exercise.name}</span>
                  <span class="ex-muscle">{draft.exercise.muscle_group}</span>
                </button>
              {:else}
                <button type="button" class="picker empty" onclick={() => openPicker(i)}>
                  <Icon icon={MagnifyingGlass} size={14} />
                  <span>Pick exercise</span>
                </button>
              {/if}
            </div>
            <label class="set-cell">
              <span class="cell-label">Weight (kg)</span>
              <input
                type="text"
                inputmode="decimal"
                bind:value={draft.weightKg}
                placeholder="80"
              />
            </label>
            <label class="set-cell">
              <span class="cell-label">Reps</span>
              <input
                type="number"
                min="1"
                max="100"
                bind:value={draft.reps}
                placeholder="8"
              />
            </label>
            <label class="set-cell">
              <span class="cell-label">RIR</span>
              <input type="number" min="0" max="10" bind:value={draft.rir} />
            </label>
            <button
              type="button"
              class="del"
              aria-label={`Remove set ${i + 1}`}
              onclick={() => removeRow(i)}
            >
              <Icon icon={Trash} size={14} />
            </button>
          </li>
        {/each}
      </ol>

      <button type="button" class="secondary" onclick={addRow}>
        <Icon icon={Plus} size={14} />
        <span>Add set</span>
      </button>

      <input type="hidden" name="sets" value={setsPayload} />

      {#if submitError}
        <p class="error" role="alert">{submitError}</p>
      {/if}

      <div class="footer-bar">
        <p class="muted">{validRowCount} valid set{validRowCount === 1 ? '' : 's'}</p>
        <button type="submit" class="primary" disabled={validRowCount === 0}>
          Save workout
        </button>
      </div>
    </form>
  {/if}

  {#if activeRow !== null}
    <div
      class="modal-backdrop"
      role="presentation"
      onclick={() => (activeRow = null)}
      onkeydown={(e) => {
        if (e.key === 'Escape') activeRow = null;
      }}
    >
      <div
        class="modal"
        role="dialog"
        aria-label="Pick exercise"
        aria-modal="true"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <label class="search">
          <span class="sr-only">Search exercises</span>
          <input
            type="search"
            bind:value={query}
            placeholder="Search (FTS5)…"
            autofocus
            aria-label="Search exercises"
          />
        </label>
        {#if suggestions.length === 0}
          <p class="muted">No matches.</p>
        {:else}
          <ul class="suggestions">
            {#each suggestions as ex (ex.id)}
              <li>
                <button type="button" onclick={() => pickExercise(ex)}>
                  <span class="name">{ex.name}</span>
                  <span class="muted">{ex.muscle_group}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {/if}
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
  h2 { font-size: var(--text-lg); font-weight: 700; }

  .form {
    display: grid;
    gap: var(--space-4);
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .field { display: grid; gap: var(--space-1); }
  .field > span,
  .cell-label {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  input[type='text'],
  input[type='number'],
  input[type='search'] {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    width: 100%;
  }

  .sets { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .set-row {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .set-cell { display: grid; gap: 2px; }
  .picker {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    color: var(--color-fg-muted);
    text-align: left;
    justify-content: flex-start;
  }
  .picker.chosen {
    border-style: solid;
    color: var(--color-fg);
    display: grid;
    gap: 2px;
    text-align: left;
  }
  .ex-name { font-weight: 600; }
  .ex-muscle {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .del {
    width: 32px;
    height: 32px;
    align-self: end;
    justify-self: end;
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .del:hover { color: var(--color-danger); background: var(--color-bg-sunken); }

  .secondary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--text-sm);
    justify-self: start;
  }
  .secondary:hover { border-color: var(--color-border-strong); }

  .primary {
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 700;
  }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .primary:hover:enabled { background: var(--color-accent-hover); }

  .footer-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .muted { color: var(--color-fg-muted); font-size: var(--text-sm); }

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

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: hsl(0 0% 0% / 0.5);
    display: grid;
    place-items: center;
    padding: var(--space-4);
    z-index: var(--z-modal);
  }
  .modal {
    width: 100%;
    max-width: 480px;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    display: grid;
    gap: var(--space-3);
    max-height: 80vh;
    overflow-y: auto;
  }
  .suggestions { list-style: none; padding: 0; margin: 0; display: grid; gap: 2px; }
  .suggestions button {
    width: 100%;
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    display: grid;
    gap: 2px;
  }
  .suggestions button:hover { background: var(--color-bg-sunken); }
  .name { font-weight: 600; }
  .sr-only {
    position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0;
  }

  @media (min-width: 768px) {
    .set-row {
      grid-template-columns: 1fr 100px 80px 80px 36px;
      align-items: end;
    }
  }
</style>

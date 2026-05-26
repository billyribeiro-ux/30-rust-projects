<script lang="ts">
  import { untrack } from 'svelte';
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import { Play, Pause, ArrowCounterClockwise } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SessionRow from '$lib/components/SessionRow.svelte';
  import { playChime } from '$lib/chime';
  import { formatClock, formatMinutes } from '$lib/format';
  import { KIND_LABEL, KIND_SECONDS, KIND_COLOR, type SessionKind } from '$lib/types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // ---- reactive UI state ----
  let mode = $state<SessionKind>('work');
  let label = $state('');
  let running = $state(false);
  let elapsedMs = $state(0);
  let startedAt: string | null = $state(null);

  // ---- recording forms (hidden) ----
  let recordForm: HTMLFormElement | undefined = $state();
  let recordPayload = $state({
    kind: 'work' as SessionKind,
    label: '',
    planned_seconds: 0,
    actual_seconds: 0,
    started_at: '',
    ended_at: ''
  });

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');

  // ---- derived clock + progress ----
  const totalSec = $derived(KIND_SECONDS[mode]);
  const remainingSec = $derived(Math.max(0, totalSec - Math.floor(elapsedMs / 1000)));
  const progress = $derived(Math.min(1, elapsedMs / (totalSec * 1000)));

  // ---- the timer loop ----
  // The $effect runs whenever `running` changes. When running becomes true we
  // schedule a requestAnimationFrame loop. The returned cleanup function
  // cancels the RAF when the effect re-runs (running becomes false) OR when
  // the component unmounts. This is the canonical "$effect with cleanup".
  //
  // We use `untrack()` to read `elapsedMs` once at startup without making the
  // effect depend on it — otherwise every tick would trigger an effect re-run
  // and the RAF would be cancelled mid-frame.
  $effect(() => {
    if (!running) return;

    let raf = 0;
    const startPerf = performance.now();
    const startElapsed = untrack(() => elapsedMs);
    const totalMs = untrack(() => KIND_SECONDS[mode]) * 1000;

    function tick(now: number) {
      const next = startElapsed + (now - startPerf);
      if (next >= totalMs) {
        elapsedMs = totalMs;
        running = false;
        finishSession();
        return;
      }
      elapsedMs = next;
      raf = requestAnimationFrame(tick);
    }

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  // Capture started_at on the first tick of a new run.
  $effect(() => {
    if (running && !startedAt) {
      startedAt = new Date().toISOString();
    }
  });

  // $inspect logs to the browser console whenever these values change. Strip
  // before shipping (or guard with import.meta.env.DEV). Useful while learning.
  $inspect('pomodoro', { mode, running, remainingSec });

  // ---- actions ----
  function toggleRunning() {
    running = !running;
  }

  function resetTimer() {
    running = false;
    elapsedMs = 0;
    startedAt = null;
  }

  function chooseMode(next: SessionKind) {
    running = false;
    elapsedMs = 0;
    startedAt = null;
    mode = next;
  }

  function finishSession() {
    playChime();
    const ended = new Date().toISOString();
    recordPayload = {
      kind: mode,
      label: label.trim(),
      planned_seconds: KIND_SECONDS[mode],
      actual_seconds: Math.floor(elapsedMs / 1000),
      started_at: startedAt ?? ended,
      ended_at: ended
    };
    queueMicrotask(() => recordForm?.requestSubmit());
    startedAt = null;
    elapsedMs = 0;
  }

  function handleDelete(id: string) {
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }

  // ---- keyboard shortcuts (svelte:window below) ----
  function handleKey(e: KeyboardEvent) {
    // Ignore when the user is typing in an input or textarea.
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;

    switch (e.key) {
      case ' ':
        e.preventDefault();
        toggleRunning();
        break;
      case 'r':
      case 'R':
        e.preventDefault();
        resetTimer();
        break;
      case '1':
        chooseMode('work');
        break;
      case '2':
        chooseMode('short_break');
        break;
      case '3':
        chooseMode('long_break');
        break;
    }
  }

  // ---- aggregate ----
  const focusMinutesToday = $derived(Math.round(data.stats.focus_seconds_today / 60));
  const focusMinutesWeek = $derived(Math.round(data.stats.focus_seconds_week / 60));
</script>

<svelte:window onkeydown={handleKey} />

<main>
  <section
    class="timer"
    aria-label="Pomodoro timer"
    style="--accent: {KIND_COLOR[mode]}; --progress: {progress};"
  >
    <div class="modes" role="tablist" aria-label="Session mode">
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'work'}
        class:active={mode === 'work'}
        onclick={() => chooseMode('work')}
      >Focus</button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'short_break'}
        class:active={mode === 'short_break'}
        onclick={() => chooseMode('short_break')}
      >Short break</button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'long_break'}
        class:active={mode === 'long_break'}
        onclick={() => chooseMode('long_break')}
      >Long break</button>
    </div>

    <div class="dial" aria-hidden="true">
      <div class="ring"></div>
    </div>

    <p class="clock" aria-live="polite" aria-atomic="true">
      <span class="time" data-testid="clock">{formatClock(remainingSec)}</span>
      <span class="mode-label">{KIND_LABEL[mode]}</span>
    </p>

    <label class="label-input">
      <span class="sr-only">Session label (optional)</span>
      <input
        type="text"
        name="label"
        bind:value={label}
        placeholder="What are you working on?"
        maxlength="120"
        autocomplete="off"
      />
    </label>

    <div class="controls">
      <button
        type="button"
        class="primary"
        aria-label={running ? 'Pause timer' : 'Start timer'}
        onclick={toggleRunning}
        data-testid="toggle"
      >
        <Icon icon={running ? Pause : Play} size={22} weight="fill" />
        <span>{running ? 'Pause' : 'Start'}</span>
      </button>
      <button
        type="button"
        class="ghost"
        aria-label="Reset timer"
        onclick={resetTimer}
        data-testid="reset"
      >
        <Icon icon={ArrowCounterClockwise} size={18} weight="bold" />
        <span>Reset</span>
      </button>
    </div>

    <p class="hint">
      Shortcuts: <kbd>Space</kbd> start/pause · <kbd>R</kbd> reset · <kbd>1</kbd>/<kbd>2</kbd>/<kbd>3</kbd> mode
    </p>
  </section>

  <dl class="stats" aria-label="Today's totals">
    <div>
      <dt>Focused today</dt>
      <dd>{focusMinutesToday} min</dd>
    </div>
    <div>
      <dt>Pomodoros today</dt>
      <dd>{data.stats.pomodoros_today}</dd>
    </div>
    <div>
      <dt>This week</dt>
      <dd>{formatMinutes(focusMinutesWeek * 60)}</dd>
    </div>
  </dl>

  <section class="history" aria-label="Recent sessions">
    <h2>Recent sessions</h2>
    {#if data.sessions.length === 0}
      <p class="empty">No sessions yet. Hit Start to begin your first pomodoro.</p>
    {:else}
      <ul>
        {#each data.sessions as session (session.id)}
          <SessionRow {session} onDelete={handleDelete} />
        {/each}
      </ul>
    {/if}
  </section>

  <!-- Hidden recording form: submitted programmatically when a session completes -->
  <form
    bind:this={recordForm}
    method="POST"
    action="?/record"
    use:enhance={() => async ({ update }) => {
      // After a record, reload the page data so stats + history refresh.
      await update();
      await invalidateAll();
    }}
    hidden
  >
    <input type="hidden" name="kind" value={recordPayload.kind} />
    <input type="hidden" name="label" value={recordPayload.label} />
    <input type="hidden" name="planned_seconds" value={recordPayload.planned_seconds} />
    <input type="hidden" name="actual_seconds" value={recordPayload.actual_seconds} />
    <input type="hidden" name="started_at" value={recordPayload.started_at} />
    <input type="hidden" name="ended_at" value={recordPayload.ended_at} />
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
</main>

<style>
  main {
    max-width: var(--container-md);
    margin-inline: auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-8);
  }

  .timer {
    display: grid;
    gap: var(--space-4);
    justify-items: center;
    text-align: center;
    padding: var(--space-8) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
  }

  .modes {
    display: inline-flex;
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
    padding: 2px;
  }

  .modes button {
    padding: var(--space-2) var(--space-3);
    color: var(--color-fg-muted);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: 500;
  }

  .modes button.active {
    background: var(--color-bg);
    color: var(--accent);
    box-shadow: var(--shadow-sm);
  }

  .dial {
    --size: clamp(200px, 50vw, 280px);
    position: relative;
    width: var(--size);
    height: var(--size);
    margin: var(--space-2) 0;
  }

  .ring {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background:
      conic-gradient(var(--accent) calc(var(--progress) * 360deg), var(--color-bg-sunken) 0);
    /* Inset ring effect */
    mask: radial-gradient(circle, transparent 58%, #000 60%);
    -webkit-mask: radial-gradient(circle, transparent 58%, #000 60%);
    transition: background var(--duration-fast) linear;
  }

  .clock {
    position: relative;
    margin-top: calc(-1 * clamp(180px, 47vw, 260px));
    display: grid;
    gap: var(--space-1);
  }

  .time {
    font-size: clamp(2.5rem, 9vw, 4rem);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: var(--accent);
  }

  .mode-label {
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--text-xs);
  }

  .label-input {
    width: min(420px, 100%);
    display: block;
  }

  .label-input input {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    text-align: center;
  }

  .controls {
    display: flex;
    gap: var(--space-3);
    flex-wrap: wrap;
    justify-content: center;
  }

  .primary,
  .ghost {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--text-base);
  }

  .primary {
    background: var(--accent);
    color: white;
    min-width: 144px;
    justify-content: center;
  }

  .primary:hover {
    filter: brightness(1.05);
  }

  .ghost {
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .ghost:hover {
    border-color: var(--color-border-strong);
  }

  .hint {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
  }

  .hint kbd {
    font-family: var(--font-mono);
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
    font-size: 0.9em;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-4);
    padding: var(--space-5);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .stats > div {
    display: grid;
    gap: 2px;
    text-align: center;
  }

  .stats dt {
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .stats dd {
    font-size: var(--text-2xl);
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .history h2 {
    font-size: var(--text-lg);
    font-weight: 600;
    margin-bottom: var(--space-3);
  }

  .history ul {
    list-style: none;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }

  .empty {
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--color-fg-muted);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-md);
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
  }

  @media (prefers-reduced-motion: reduce) {
    .ring {
      transition: none;
    }
  }
</style>

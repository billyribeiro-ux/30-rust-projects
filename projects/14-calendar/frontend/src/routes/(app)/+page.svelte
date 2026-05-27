<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import type { Calendar, Occurrence } from '$lib/types';

  let { data, form }: PageProps = $props();

  // Anchor month from data.month (YYYY-MM).
  const anchor = $derived(new Date(`${data.month}-01T00:00:00Z`));
  const monthLabel = $derived(
    anchor.toLocaleString('en-US', { month: 'long', year: 'numeric', timeZone: 'UTC' })
  );

  const prevMonth = $derived(prevMonthString(data.month));
  const nextMonth = $derived(nextMonthString(data.month));

  function prevMonthString(m: string): string {
    const [y, mm] = m.split('-').map(Number);
    const d = new Date(Date.UTC(y!, mm! - 2, 1));
    return d.toISOString().slice(0, 7);
  }
  function nextMonthString(m: string): string {
    const [y, mm] = m.split('-').map(Number);
    const d = new Date(Date.UTC(y!, mm!, 1));
    return d.toISOString().slice(0, 7);
  }

  // Build a 6-row × 7-col grid for the month view. Sunday-first.
  type Cell = { date: Date; iso: string; inMonth: boolean; events: Occurrence[] };
  const grid = $derived.by((): Cell[] => {
    const first = new Date(anchor);
    const dayOfWeek = first.getUTCDay(); // 0 = Sunday
    const gridStart = new Date(first);
    gridStart.setUTCDate(first.getUTCDate() - dayOfWeek);
    const cells: Cell[] = [];
    for (let i = 0; i < 42; i++) {
      const d = new Date(gridStart);
      d.setUTCDate(gridStart.getUTCDate() + i);
      const iso = d.toISOString().slice(0, 10);
      const inMonth = d.getUTCMonth() === first.getUTCMonth();
      const events = data.occurrences.filter((o) => o.start_at.slice(0, 10) === iso);
      cells.push({ date: d, iso, inMonth, events });
    }
    return cells;
  });

  function colorFor(calendarId: string): string {
    const c = data.calendars.find((c: Calendar) => c.id === calendarId);
    return c?.color ?? '#3b82f6';
  }
  function fmtTime(iso: string): string {
    return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  let showCreateCal = $state(false);
  let showCreateEvent = $state(false);

  // Reset show-flags after a successful submit (form actions return ok=true).
  $effect(() => {
    if (form && 'ok' in form && form.ok) {
      showCreateCal = false;
      showCreateEvent = false;
    }
  });

  // Reasonable defaults for the event form (today 9am → 10am local).
  function defaultStart(): string {
    const d = new Date();
    d.setHours(9, 0, 0, 0);
    return toDatetimeLocal(d);
  }
  function defaultEnd(): string {
    const d = new Date();
    d.setHours(10, 0, 0, 0);
    return toDatetimeLocal(d);
  }
  function toDatetimeLocal(d: Date): string {
    // datetime-local input format: YYYY-MM-DDTHH:MM in local time
    const pad = (n: number) => n.toString().padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
</script>

<svelte:head><title>{monthLabel} — Calendar</title></svelte:head>

<main>
  <header>
    <h1>{monthLabel}</h1>
    <p class="lead">Hi, {data.user?.name || data.user?.email}.</p>
    <div class="actions">
      <a class="nav" href="/?m={prevMonth}" aria-label="Previous month">← Prev</a>
      <a class="nav" href="/" aria-label="This month">Today</a>
      <a class="nav" href="/?m={nextMonth}" aria-label="Next month">Next →</a>
      <button type="button" class="ghost" onclick={() => (showCreateCal = !showCreateCal)}>
        + Calendar
      </button>
      <button type="button" class="primary" disabled={data.calendars.length === 0}
              onclick={() => (showCreateEvent = !showCreateEvent)}>
        + Event
      </button>
    </div>
  </header>

  {#if data.calendars.length === 0}
    <p class="empty">You don't have any calendars yet. Create one to add events.</p>
  {/if}

  {#if showCreateCal}
    <section class="panel" aria-label="Create calendar">
      <h2>Create calendar</h2>
      <form
        method="POST"
        action="?/newCalendar"
        use:enhance
      >
        <label class="field">
          <span>Name <em>*</em></span>
          <input type="text" name="name" required maxlength="200" value={(form?.name as string | undefined) ?? ''} />
        </label>
        <label class="field">
          <span>Color</span>
          <input type="color" name="color" value="#3b82f6" />
        </label>
        <label class="field">
          <span>Default timezone</span>
          <input type="text" name="tz" value={Intl.DateTimeFormat().resolvedOptions().timeZone} />
        </label>
        {#if form && 'error' in form && form.error}
          <p class="error" role="alert">{form.error}</p>
        {/if}
        <button type="submit" class="primary">Create calendar</button>
      </form>
    </section>
  {/if}

  {#if showCreateEvent && data.calendars.length > 0}
    <section class="panel" aria-label="Create event">
      <h2>Create event</h2>
      <form
        method="POST"
        action="?/newEvent"
        use:enhance
      >
        <label class="field">
          <span>Calendar <em>*</em></span>
          <select name="calendar_id" required>
            {#each data.calendars.filter((c: Calendar) => c.permission !== 'view') as c (c.id)}
              <option value={c.id}>{c.name}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Title <em>*</em></span>
          <input type="text" name="title" required maxlength="200" value={(form?.title as string | undefined) ?? ''} />
        </label>
        <label class="field">
          <span>Start <em>*</em></span>
          <input type="datetime-local" name="start" required value={(form?.start as string | undefined) ?? defaultStart()} />
        </label>
        <label class="field">
          <span>End <em>*</em></span>
          <input type="datetime-local" name="end" required value={(form?.end as string | undefined) ?? defaultEnd()} />
        </label>
        <label class="field">
          <span>Timezone</span>
          <input type="text" name="tz" value={Intl.DateTimeFormat().resolvedOptions().timeZone} />
        </label>
        <label class="field">
          <span>Recurrence (RRULE — optional)</span>
          <input
            type="text"
            name="rrule"
            placeholder="RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR"
          />
        </label>
        {#if form && 'eventError' in form && form.eventError}
          <p class="error" role="alert">{form.eventError}</p>
        {/if}
        <button type="submit" class="primary">Save event</button>
      </form>
    </section>
  {/if}

  <ol class="weekdays" aria-hidden="true">
    <li>Sun</li><li>Mon</li><li>Tue</li><li>Wed</li><li>Thu</li><li>Fri</li><li>Sat</li>
  </ol>

  <div class="month-grid" aria-label={monthLabel}>
    {#each grid as cell (cell.iso)}
      <div class="cell" class:other-month={!cell.inMonth}>
        <div class="day-num"><time datetime={cell.iso}>{cell.date.getUTCDate()}</time></div>
        {#if cell.events.length > 0}
          <ul class="events">
            {#each cell.events.slice(0, 3) as e (e.event_id + e.start_at)}
              <li
                class="event"
                style:--color={colorFor(e.calendar_id)}
                title="{e.title} — {fmtTime(e.start_at)}"
              >
                {#if !e.all_day}<time datetime={e.start_at}>{fmtTime(e.start_at)}</time>{/if}
                <span class="title">{e.title}</span>
                {#if e.is_recurring}<span class="badge" aria-label="recurring">↻</span>{/if}
              </li>
            {/each}
            {#if cell.events.length > 3}
              <li class="more">+{cell.events.length - 3} more</li>
            {/if}
          </ul>
        {/if}
      </div>
    {/each}
  </div>
</main>

<style>
  main {
    max-width: var(--container-xl, 1280px);
    margin-inline: auto;
    padding: var(--space-5) var(--space-4);
    display: grid;
    gap: var(--space-4);
  }
  header {
    display: grid;
    gap: var(--space-2);
  }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .actions { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; }
  .nav {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg-muted);
    text-decoration: none;
    font-size: var(--text-sm);
    border-radius: var(--radius-sm);
  }
  .nav:hover { background: var(--color-bg-elev); color: var(--color-fg); }
  .primary, .ghost {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    font-weight: 600;
    margin-left: auto;
  }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .ghost {
    color: var(--color-fg);
    border: 1px solid var(--color-border);
  }
  .empty {
    padding: var(--space-3);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-sm);
    color: var(--color-fg-muted);
    text-align: center;
  }
  .panel {
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  form { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field em { color: var(--color-danger); font-style: normal; }
  .field input, .field select {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  .error {
    padding: var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-sm);
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    list-style: none;
    padding: 0;
    margin: 0;
    color: var(--color-fg-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: center;
  }
  .weekdays li { padding: var(--space-1); }

  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(6, minmax(80px, 1fr));
    gap: 1px;
    background: var(--color-border);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  .cell {
    background: var(--color-bg);
    padding: var(--space-1) var(--space-2);
    display: grid;
    gap: 2px;
    align-content: start;
    min-height: 80px;
  }
  .cell.other-month { background: var(--color-bg-sunken); color: var(--color-fg-muted); }
  .day-num { font-size: var(--text-xs); font-weight: 600; }
  .events { list-style: none; padding: 0; margin: 0; display: grid; gap: 2px; }
  .event {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color, var(--color-accent)) 18%, transparent);
    border-left: 3px solid var(--color, var(--color-accent));
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .event time { font-variant-numeric: tabular-nums; color: var(--color-fg-muted); }
  .event .title { overflow: hidden; text-overflow: ellipsis; }
  .event .badge { font-size: 10px; opacity: 0.7; }
  .more { font-size: var(--text-xs); color: var(--color-fg-muted); }

  @media (max-width: 768px) {
    .month-grid { grid-template-rows: repeat(6, minmax(60px, 1fr)); }
    .event { font-size: 10px; }
    .event time { display: none; }
  }
  @media (max-width: 480px) {
    .cell { padding: 2px; min-height: 50px; }
    .events { display: none; }
    .day-num::after { content: '·'; margin-left: 2px; color: var(--color-accent); }
    .cell:not(:has(.events)) .day-num::after { display: none; }
  }
</style>

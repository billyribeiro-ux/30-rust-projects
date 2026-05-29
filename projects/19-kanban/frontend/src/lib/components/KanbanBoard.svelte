<script lang="ts">
  import { flip } from 'svelte/animate';
  import { cardsApi, listsApi, ApiCallError } from '$lib/api';
  import type { BoardFull, CardRow, ListRow, Role } from '$lib/types';
  import { animateDrop, prefersReducedMotion } from '$lib/motion';

  type Props = {
    board: BoardFull;
  };

  let { board: initial }: Props = $props();

  // Local optimistic state, seeded once from SSR. The page is bound to
  // a single board instance for its lifetime — re-seeding on prop change
  // would discard in-flight optimistic mutations, which is exactly what
  // we don't want. Silenced warnings document the intent.
  // svelte-ignore state_referenced_locally
  let lists = $state<ListRow[]>(initial.lists.map((l) => ({ ...l })));
  // svelte-ignore state_referenced_locally
  let cards = $state<CardRow[]>(initial.cards.map((c) => ({ ...c })));
  // svelte-ignore state_referenced_locally
  const role: Role = initial.role;
  const canEdit = role !== 'viewer';

  let dragging = $state<string | null>(null); // card_id being dragged
  let dragOriginList = $state<string | null>(null);
  let error = $state<string | null>(null);

  // Build per-list ordered card arrays. Reactive via $derived.
  let cardsByList = $derived.by(() => {
    const m = new Map<string, CardRow[]>();
    for (const l of lists) m.set(l.id, []);
    for (const c of cards) m.get(c.list_id)?.push(c);
    for (const arr of m.values()) arr.sort((a, b) => a.position - b.position);
    return m;
  });

  let sortedLists = $derived([...lists].sort((a, b) => a.position - b.position));

  function midpoint(list_id: string, beforeIdx: number): number {
    const arr = cardsByList.get(list_id) ?? [];
    const prev = beforeIdx > 0 ? arr[beforeIdx - 1] : null;
    const next = beforeIdx < arr.length ? arr[beforeIdx] : null;
    if (!prev && !next) return 1024;
    if (!prev && next) return next.position - 1024;
    if (prev && !next) return prev.position + 1024;
    return ((prev?.position ?? 0) + (next?.position ?? 0)) / 2;
  }

  function onDragStart(e: DragEvent, cardId: string, listId: string) {
    if (!canEdit) {
      e.preventDefault();
      return;
    }
    dragging = cardId;
    dragOriginList = listId;
    e.dataTransfer?.setData('text/plain', cardId);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function onDragOver(e: DragEvent) {
    if (!canEdit || !dragging) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  }

  async function onDrop(e: DragEvent, targetListId: string, beforeIdx: number) {
    if (!canEdit || !dragging) return;
    e.preventDefault();
    const cardId = dragging;
    dragging = null;
    const originList = dragOriginList;
    dragOriginList = null;
    const card = cards.find((c) => c.id === cardId);
    if (!card) return;

    // No-op drop on same position.
    if (originList === targetListId) {
      const arr = cardsByList.get(targetListId) ?? [];
      const currentIdx = arr.findIndex((c) => c.id === cardId);
      if (currentIdx === beforeIdx || currentIdx + 1 === beforeIdx) return;
    }

    const newPos = midpoint(targetListId, beforeIdx);
    const prevState = { list_id: card.list_id, position: card.position };

    // Optimistic.
    card.list_id = targetListId;
    card.position = newPos;
    cards = [...cards];

    // GSAP drop physics (reduced-motion safe).
    if (!prefersReducedMotion()) {
      const el = document.querySelector<HTMLElement>(`[data-card-id="${cardId}"]`);
      if (el) animateDrop(el);
    }

    try {
      await cardsApi.update(fetch, cardId, {
        list_id: targetListId,
        position: newPos
      });
    } catch (err) {
      // Rollback.
      card.list_id = prevState.list_id;
      card.position = prevState.position;
      cards = [...cards];
      error =
        err instanceof ApiCallError
          ? `Move failed: ${err.message}`
          : 'Move failed: network error.';
      setTimeout(() => (error = null), 4000);
    }
  }

  async function addCard(listId: string, title: string) {
    if (!canEdit || !title.trim()) return;
    const arr = cardsByList.get(listId) ?? [];
    const tail = arr[arr.length - 1];
    const pos = tail ? tail.position + 1024 : 1024;
    try {
      const created = await listsApi.createCard(fetch, listId, title.trim(), pos);
      cards = [...cards, created];
    } catch (err) {
      error =
        err instanceof ApiCallError
          ? `Add card failed: ${err.message}`
          : 'Add card failed.';
      setTimeout(() => (error = null), 4000);
    }
  }
</script>

{#if error}
  <p class="banner" role="alert">{error}</p>
{/if}

<div class="board" aria-label={`Board ${initial.name}`}>
  {#each sortedLists as list (list.id)}
    <section class="list" aria-label={`List ${list.name}`}>
      <header>
        <h2>{list.name}</h2>
        <span class="count">{cardsByList.get(list.id)?.length ?? 0}</span>
      </header>

      <ul
        class="cards"
        ondragover={onDragOver}
        ondrop={(e) => onDrop(e, list.id, cardsByList.get(list.id)?.length ?? 0)}
      >
        {#each cardsByList.get(list.id) ?? [] as card, idx (card.id)}
          <li
            class="card"
            class:dragging={dragging === card.id}
            data-card-id={card.id}
            draggable={canEdit}
            ondragstart={(e) => onDragStart(e, card.id, list.id)}
            ondragover={onDragOver}
            ondrop={(e) => {
              e.stopPropagation();
              onDrop(e, list.id, idx);
            }}
            animate:flip={{ duration: 220 }}
          >
            <span class="title">{card.title}</span>
            {#if card.due_at}
              <time class="due" datetime={card.due_at}>
                {new Date(card.due_at).toLocaleDateString()}
              </time>
            {/if}
          </li>
        {/each}
      </ul>

      {#if canEdit}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            const fd = new FormData(e.currentTarget);
            const title = String(fd.get('title') ?? '');
            if (title) {
              addCard(list.id, title);
              (e.currentTarget as HTMLFormElement).reset();
            }
          }}
        >
          <input type="text" name="title" placeholder="+ Add card" aria-label="New card title" />
        </form>
      {/if}
    </section>
  {/each}
</div>

<style>
  .banner {
    margin: var(--space-3) var(--space-4) 0;
    padding: var(--space-2) var(--space-3);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    color: var(--color-danger);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  .board {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(260px, 320px);
    gap: var(--space-4);
    overflow-x: auto;
    padding: var(--space-4);
    min-height: calc(100dvh - 64px);
  }
  .list {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    height: fit-content;
    max-height: 100%;
  }
  .list header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .list h2 {
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--color-fg-muted);
    letter-spacing: 0.04em;
  }
  .count {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
    padding: 0 var(--space-2);
    background: var(--color-bg-sunken);
    border-radius: var(--radius-pill);
  }
  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-2);
    min-height: var(--space-6);
  }
  .card {
    padding: var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: grab;
    display: grid;
    gap: var(--space-1);
    transition: border-color var(--duration-fast) var(--ease-out);
  }
  .card:hover {
    border-color: var(--color-border-strong);
  }
  .card.dragging {
    opacity: 0.3;
  }
  .title {
    font-weight: 500;
  }
  .due {
    font-size: var(--text-xs);
    color: var(--color-fg-muted);
  }
  form input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  form input:focus {
    outline: 2px solid var(--color-accent);
    color: var(--color-fg);
    background: var(--color-bg);
  }
</style>

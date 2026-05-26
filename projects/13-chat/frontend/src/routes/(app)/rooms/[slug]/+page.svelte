<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { wsUrl } from '$lib/api';
  import type { Message, RoomEvent } from '$lib/types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // Initial scrollback snapshot. The WS stream appends new arrivals; if you
  // navigate to a different room (same component, new data), an $effect
  // below resets this list to the new initialMessages.
  let messages = $state<Message[]>([]);
  let presence = $state<Set<string>>(new Set());
  let connected = $state(false);
  let composer = $state('');
  let sending = $state(false);
  let scroller: HTMLDivElement;

  let ws: WebSocket | null = null;

  // Reconnect with simple backoff. Browser-only ($effect runs after hydration).
  let backoff = 500;
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

  function connect() {
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) return;
    ws = new WebSocket(wsUrl(data.room.slug));
    ws.addEventListener('open', () => {
      connected = true;
      backoff = 500;
    });
    ws.addEventListener('message', async (e) => {
      let ev: RoomEvent;
      try {
        ev = JSON.parse(e.data) as RoomEvent;
      } catch {
        return;
      }
      switch (ev.type) {
        case 'message':
          messages = [...messages, ev];
          await tick();
          scrollToBottom();
          break;
        case 'joined':
          presence = new Set([...presence, ev.user_id]);
          break;
        case 'left':
          presence = new Set([...presence].filter((id) => id !== ev.user_id));
          break;
      }
    });
    ws.addEventListener('close', () => {
      connected = false;
      ws = null;
      reconnectTimer = setTimeout(connect, backoff);
      backoff = Math.min(backoff * 2, 10_000);
    });
    ws.addEventListener('error', () => ws?.close());
  }

  function send() {
    const body = composer.trim();
    if (!body || !ws || ws.readyState !== WebSocket.OPEN) return;
    sending = true;
    ws.send(JSON.stringify({ type: 'send', body }));
    composer = '';
    sending = false;
  }

  function scrollToBottom() {
    if (!scroller) return;
    scroller.scrollTop = scroller.scrollHeight;
  }

  // Reset the message list when navigating to a different room (so the same
  // component, with new params, doesn't show the old room's scrollback).
  $effect(() => {
    messages = [...data.initialMessages];
    tick().then(scrollToBottom);
  });

  $effect(() => {
    connect();
    return () => {
      if (reconnectTimer) clearTimeout(reconnectTimer);
      ws?.close();
    };
  });

  onDestroy(() => {
    if (reconnectTimer) clearTimeout(reconnectTimer);
    ws?.close();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function fmtTime(iso: string): string {
    return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<svelte:head><title>#{data.room.slug} — Chat</title></svelte:head>

<div class="room">
  <header class="room-head">
    <div class="who">
      <a href="/" class="back" aria-label="Back to rooms">←</a>
      <h1>#{data.room.slug}</h1>
      <span class="name">{data.room.name}</span>
    </div>
    <div class="status" aria-live="polite">
      <span class="dot" class:on={connected} class:off={!connected}></span>
      <span class="status-label">{connected ? 'Live' : 'Reconnecting…'}</span>
      <span class="presence">· {presence.size} online</span>
    </div>
  </header>

  <div class="scroller" bind:this={scroller} role="log" aria-live="polite">
    {#if messages.length === 0}
      <p class="empty">No messages yet. Be the first to say something.</p>
    {:else}
      <ol class="messages">
        {#each messages as m (m.id)}
          <li>
            <div class="meta">
              <span class="author">{m.author}</span>
              <time datetime={m.created_at}>{fmtTime(m.created_at)}</time>
            </div>
            <div class="body">{m.body}</div>
          </li>
        {/each}
      </ol>
    {/if}
  </div>

  <form
    class="composer"
    onsubmit={(e) => {
      e.preventDefault();
      send();
    }}
  >
    <label class="sr-only" for="composer-input">Message</label>
    <textarea
      id="composer-input"
      bind:value={composer}
      onkeydown={onKey}
      rows="1"
      placeholder="Message #{data.room.slug}…"
      maxlength="2000"
      disabled={!connected}
    ></textarea>
    <button type="submit" class="send" disabled={!connected || sending || !composer.trim()}>
      Send
    </button>
  </form>
</div>

<style>
  .room {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: calc(100dvh - var(--topnav-height, 56px));
    max-width: var(--container-lg);
    margin-inline: auto;
    width: 100%;
  }
  .room-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border-bottom: 1px solid var(--color-border);
    flex-wrap: wrap;
  }
  .who { display: flex; align-items: center; gap: var(--space-3); }
  .back { color: var(--color-fg-muted); font-size: var(--text-lg); text-decoration: none; }
  .back:hover { color: var(--color-fg); }
  h1 {
    font-size: var(--text-lg);
    font-weight: 700;
    font-family: var(--font-mono, monospace);
    color: var(--color-accent);
  }
  .name { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .status { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-sm); color: var(--color-fg-muted); }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--color-fg-muted);
  }
  .dot.on { background: hsl(142 71% 45%); box-shadow: 0 0 6px hsl(142 71% 45% / 0.5); }
  .dot.off { background: hsl(38 92% 50%); animation: pulse 1.5s ease-in-out infinite; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.off { animation: none; }
  }
  .presence { font-variant-numeric: tabular-nums; }

  .scroller {
    overflow-y: auto;
    padding: var(--space-4);
    background: var(--color-bg);
    scroll-behavior: smooth;
  }
  @media (prefers-reduced-motion: reduce) {
    .scroller { scroll-behavior: auto; }
  }
  .empty { color: var(--color-fg-muted); text-align: center; padding: var(--space-6); }
  .messages { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-3); }
  .meta {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    font-size: var(--text-xs);
  }
  .author { font-weight: 600; color: var(--color-fg); }
  .meta time { color: var(--color-fg-muted); }
  .body {
    margin-top: 2px;
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-wrap: anywhere;
    line-height: var(--leading-relaxed, 1.5);
  }

  .composer {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border-top: 1px solid var(--color-border);
  }
  textarea {
    flex: 1;
    resize: none;
    min-height: 40px;
    max-height: 200px;
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font: inherit;
    line-height: var(--leading-snug, 1.4);
  }
  textarea:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
  .send {
    padding: 0 var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
  }
  .send:disabled { opacity: 0.5; cursor: not-allowed; }
  .sr-only {
    position: absolute;
    width: 1px; height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0,0,0,0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 480px) {
    .room-head { padding: var(--space-2) var(--space-3); }
    .scroller { padding: var(--space-3); }
    .composer { padding: var(--space-2) var(--space-3); }
    .name { display: none; }
  }
</style>

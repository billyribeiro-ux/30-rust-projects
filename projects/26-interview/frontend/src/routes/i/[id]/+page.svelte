<script lang="ts">
  import { execApi, wsUrl } from '$lib/api';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  let code = $state(data.interview.code);
  // svelte-ignore state_referenced_locally
  const lang = data.interview.language;
  let output = $state('');
  let ws = $state<WebSocket | null>(null);
  let wsStatus = $state<'connecting' | 'open' | 'closed'>('connecting');

  $effect(() => {
    if (typeof WebSocket === 'undefined') return;
    const socket = new WebSocket(wsUrl(data.interview.id));
    socket.onopen = () => (wsStatus = 'open');
    socket.onclose = () => (wsStatus = 'closed');
    socket.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data) as { kind: string; value?: string };
        if (msg.kind === 'code' && typeof msg.value === 'string' && msg.value !== code) {
          code = msg.value;
        }
      } catch {
        /* ignore */
      }
    };
    ws = socket;
    return () => socket.close();
  });

  function onCodeChange(e: Event) {
    const value = (e.currentTarget as HTMLTextAreaElement).value;
    code = value;
    ws?.send(JSON.stringify({ kind: 'code', value }));
  }

  async function run() {
    try {
      const r = await execApi.run(fetch, data.interview.id, lang, code);
      output = r.stdout + (r.stderr ? `\nSTDERR:\n${r.stderr}` : '');
    } catch (err) {
      output = err instanceof Error ? err.message : 'Run failed.';
    }
  }
</script>

<svelte:head><title>Interview — {data.interview.candidate_email}</title></svelte:head>

<header>
  <a class="back" href="/">← All interviews</a>
  <h1>{data.interview.candidate_email}</h1>
  <span class={`badge status-${wsStatus}`}>WS {wsStatus}</span>
</header>

<main>
  <textarea
    class="editor"
    rows="20"
    aria-label="Code editor"
    bind:value={code}
    oninput={onCodeChange}
  ></textarea>
  <div class="controls">
    <button type="button" class="primary" onclick={run}>Run</button>
  </div>
  <pre class="output" aria-label="Output">{output || '— No output yet —'}</pre>
</main>

<style>
  header { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-4); background: var(--color-bg-elev); border-bottom: 1px solid var(--color-border); }
  .back { color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  h1 { font-size: var(--text-lg); font-weight: 700; flex: 1; }
  .badge { padding: 0 var(--space-2); border-radius: var(--radius-pill); font-size: var(--text-xs); text-transform: uppercase; background: var(--color-bg-sunken); color: var(--color-fg-muted); }
  .status-open { background: hsl(140 60% 50% / 0.18); color: hsl(140 60% 25%); }
  .status-closed { background: hsl(0 72% 51% / 0.12); color: var(--color-danger); }
  main { max-width: 960px; margin: 0 auto; padding: var(--space-4); display: grid; gap: var(--space-3); }
  .editor { width: 100%; font-family: var(--font-mono, monospace); padding: var(--space-3); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .controls { display: flex; gap: var(--space-2); }
  .primary { padding: var(--space-2) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .output { background: var(--color-bg-sunken); border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: var(--space-3); white-space: pre-wrap; min-height: 100px; font-family: var(--font-mono, monospace); font-size: var(--text-sm); }
</style>

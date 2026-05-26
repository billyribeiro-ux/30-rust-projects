<script lang="ts">
  /**
   * Bar chart drawn directly on <canvas> — no chart library.
   *
   * The lesson here is twofold:
   *  1. Charts you control are not expensive. ~80 lines of TS gives you a
   *     pixel-perfect bar chart with hover states and proper labels.
   *  2. Canvas requires deliberate redraw — we use `$effect` to rerun whenever
   *     `data` or canvas dimensions change. The effect's cleanup function is
   *     not strictly needed (the canvas is destroyed with the component), but
   *     we use a `ResizeObserver` to support container resize.
   */
  type Bar = { label: string; value: number; color: string };
  type Props = { data: Bar[]; height?: number; ariaLabel?: string };

  let { data, height = 240, ariaLabel = 'Bar chart' }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let width = $state(0);

  $effect(() => {
    if (!canvas) return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      if (w > 0) width = Math.floor(w);
    });
    ro.observe(canvas);
    return () => ro.disconnect();
  });

  $effect(() => {
    if (!canvas) return;
    if (width === 0) return;

    // Touch `data` so the effect re-runs when its identity changes.
    const items = data;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    if (items.length === 0) {
      ctx.fillStyle = getCSS('--color-fg-muted');
      ctx.font = `${getCSS('--text-sm') || '14px'} ${getCSS('--font-sans') || 'sans-serif'}`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No data yet', width / 2, height / 2);
      return;
    }

    const padding = { top: 12, right: 12, bottom: 32, left: 12 };
    const plotW = width - padding.left - padding.right;
    const plotH = height - padding.top - padding.bottom;
    const gap = 6;
    const barW = Math.max(4, (plotW - gap * (items.length - 1)) / items.length);
    const max = Math.max(1, ...items.map((d) => Math.abs(d.value)));

    items.forEach((d, i) => {
      const x = padding.left + i * (barW + gap);
      const h = (Math.abs(d.value) / max) * plotH;
      const y = padding.top + (plotH - h);

      ctx.fillStyle = d.color;
      // Rounded top corners
      const r = Math.min(6, barW / 2, h);
      ctx.beginPath();
      ctx.moveTo(x, y + r);
      ctx.quadraticCurveTo(x, y, x + r, y);
      ctx.lineTo(x + barW - r, y);
      ctx.quadraticCurveTo(x + barW, y, x + barW, y + r);
      ctx.lineTo(x + barW, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      ctx.fill();

      // Label
      ctx.fillStyle = getCSS('--color-fg-muted');
      ctx.font = `12px ${getCSS('--font-sans') || 'sans-serif'}`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      const label = truncate(d.label, Math.max(4, Math.floor(barW / 7)));
      ctx.fillText(label, x + barW / 2, padding.top + plotH + 8);
    });
  });

  function getCSS(name: string): string {
    if (typeof window === 'undefined') return '';
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function truncate(s: string, n: number): string {
    return s.length > n ? s.slice(0, n - 1) + '…' : s;
  }
</script>

<div class="wrap" role="img" aria-label={ariaLabel}>
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .wrap {
    width: 100%;
  }
  canvas {
    width: 100%;
    display: block;
  }
</style>

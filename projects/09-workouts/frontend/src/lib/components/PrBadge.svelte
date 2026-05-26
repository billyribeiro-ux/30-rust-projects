<script lang="ts">
  // The PR badge is intentionally tiny and deterministic — no animations,
  // no random colors, no time-of-day variants. That makes it usable as the
  // anchor for our Playwright visual-regression test (toHaveScreenshot).
  type Props = { label?: string };
  let { label = 'PR' }: Props = $props();
</script>

<span class="pr-badge" aria-label="Personal record" role="img">
  <span class="dot" aria-hidden="true"></span>
  <span class="text">{label}</span>
</span>

<style>
  .pr-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 999px;
    background: hsl(38 92% 45%);
    color: hsl(38 100% 10%);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    line-height: 1;
    /* Pin the box so cross-OS font fallbacks can't shift the screenshot. */
    height: 18px;
    box-sizing: border-box;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: hsl(38 100% 10%);
  }
  .text { font-variant-numeric: tabular-nums; }
</style>

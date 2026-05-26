<script lang="ts">
  import type { Member } from '$lib/types';

  type Props = { member: Member };
  let { member }: Props = $props();
</script>

<span class="pill" style="--accent: {member.color}">
  <span class="dot" aria-hidden="true"></span>
  <span class="name">{member.name}</span>
</span>

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px var(--space-2);
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    font-size: var(--text-sm);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--accent);
  }
</style>

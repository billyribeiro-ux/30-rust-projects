<script lang="ts">
  import type { Component } from 'svelte';

  type Weight = 'thin' | 'light' | 'regular' | 'bold' | 'fill' | 'duotone';

  type Props = {
    icon: Component<{ size?: number | string; color?: string; weight?: Weight }>;
    size?: number;
    weight?: Weight;
    color?: string;
    label?: string;
  };

  let {
    icon: IconComponent,
    size = 20,
    weight = 'regular',
    color = 'currentColor',
    label
  }: Props = $props();
</script>

<span class="icon-wrap" aria-hidden={label ? undefined : 'true'}>
  {#if label}<span class="sr-only">{label}</span>{/if}
  <IconComponent {size} {weight} {color} />
</span>

<style>
  .icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
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
</style>

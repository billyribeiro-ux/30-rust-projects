import type { Action } from 'svelte/action';

/**
 * Svelte action: invoke the callback when a pointer or focus event occurs
 * outside the node. Use for popovers, dropdowns, command palettes.
 *
 * Usage: `<div use:clickOutside={() => (open = false)}>...</div>`
 */
export const clickOutside: Action<HTMLElement, () => void> = (node, callback) => {
  let cb = callback;

  function handle(event: Event) {
    const target = event.target as Node | null;
    if (target && !node.contains(target)) cb();
  }

  document.addEventListener('pointerdown', handle, true);
  document.addEventListener('focusin', handle, true);

  return {
    update(next: () => void) {
      cb = next;
    },
    destroy() {
      document.removeEventListener('pointerdown', handle, true);
      document.removeEventListener('focusin', handle, true);
    }
  };
};

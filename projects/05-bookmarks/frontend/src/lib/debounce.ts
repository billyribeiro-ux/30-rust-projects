/**
 * Returns a debounced version of `fn`. Calls within `wait` ms are coalesced
 * into a single trailing call. `cancel()` clears any pending call.
 *
 * Used by the search box: we don't want to hit the API on every keystroke.
 */
export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  wait: number
): { (...args: Args): void; cancel(): void } {
  let timer: ReturnType<typeof setTimeout> | undefined;

  function debounced(...args: Args) {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      fn(...args);
    }, wait);
  }

  debounced.cancel = () => {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  return debounced;
}

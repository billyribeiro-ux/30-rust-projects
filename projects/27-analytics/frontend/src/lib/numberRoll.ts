/**
 * Animated number-roll using GSAP, lazy-loaded so the chunk isn't in
 * the critical-path bundle. Respects `prefers-reduced-motion`.
 */

let gsapPromise: Promise<typeof import('gsap')> | null = null;
function loadGsap() {
  if (!gsapPromise) gsapPromise = import('gsap');
  return gsapPromise;
}

export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') return true;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export async function rollNumber(
  el: HTMLElement,
  from: number,
  to: number,
  ms = 600
): Promise<void> {
  if (prefersReducedMotion() || ms <= 0) {
    el.textContent = formatInteger(to);
    return;
  }
  const { gsap } = await loadGsap();
  const obj = { v: from };
  gsap.killTweensOf(obj);
  await new Promise<void>((resolve) =>
    gsap.to(obj, {
      v: to,
      duration: ms / 1000,
      ease: 'power3.out',
      onUpdate() {
        el.textContent = formatInteger(obj.v);
      },
      onComplete() {
        el.textContent = formatInteger(to);
        resolve();
      }
    })
  );
}

function formatInteger(n: number): string {
  return Math.round(n).toLocaleString();
}

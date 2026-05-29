/**
 * GSAP-backed drop animation, gated on `prefers-reduced-motion`.
 *
 * GSAP is loaded lazily (dynamic import) so it doesn't bloat the
 * first paint, and so SSR doesn't try to evaluate a browser-only module.
 */

export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') return true;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

let gsapPromise: Promise<typeof import('gsap')> | null = null;
function loadGsap() {
  if (!gsapPromise) gsapPromise = import('gsap');
  return gsapPromise;
}

export async function animateDrop(el: HTMLElement): Promise<void> {
  if (prefersReducedMotion()) return;
  const { gsap } = await loadGsap();
  // A tiny tactile "settle" — squash on land, rebound to rest. Cheap, cinematic.
  gsap.killTweensOf(el);
  gsap
    .timeline()
    .fromTo(
      el,
      { scale: 1.04, y: -6, boxShadow: '0 12px 28px rgb(0 0 0 / 0.18)' },
      { scale: 1, y: 0, boxShadow: '0 1px 2px rgb(0 0 0 / 0.06)', duration: 0.28, ease: 'power3.out' }
    );
}

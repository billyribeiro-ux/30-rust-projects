/**
 * GSAP-backed scroll choreography. Lazy-imported, reduced-motion aware.
 *
 * `revealOnScroll(el)` fades + lifts the element when it scrolls into
 * the viewport. Plain IntersectionObserver fallback — we ship the
 * lesson, not the full ScrollTrigger plugin (which lives behind GSAP's
 * paid Club but the free fallback is enough).
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

export async function revealOnScroll(el: HTMLElement, opts: { y?: number; duration?: number } = {}) {
  if (typeof window === 'undefined') return;
  if (prefersReducedMotion()) {
    el.style.opacity = '1';
    return;
  }
  const { gsap } = await loadGsap();
  gsap.set(el, { opacity: 0, y: opts.y ?? 36 });
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          gsap.to(el, {
            opacity: 1,
            y: 0,
            duration: opts.duration ?? 0.6,
            ease: 'power3.out'
          });
          io.disconnect();
        }
      }
    },
    { threshold: 0.15 }
  );
  io.observe(el);
}

export async function heroIntro(el: HTMLElement) {
  if (prefersReducedMotion()) {
    el.style.opacity = '1';
    return;
  }
  const { gsap } = await loadGsap();
  gsap.from(el, { opacity: 0, y: 64, duration: 0.9, ease: 'power3.out' });
}

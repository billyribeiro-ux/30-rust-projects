// Synthesized chime using the Web Audio API — no audio file needed.
// Plays a short two-note tone (A5 then E5) that decays exponentially.
//
// Browsers block AudioContext creation until a user gesture; we only call
// this from button-click or keyboard handlers, never from a timer.

let ctx: AudioContext | null = null;

function getCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctor) return null;
  if (!ctx) ctx = new Ctor();
  return ctx;
}

export function playChime(): void {
  const c = getCtx();
  if (!c) return;
  // If the page has been suspended (mobile), resume on demand.
  if (c.state === 'suspended') void c.resume();

  playTone(c, 880, c.currentTime, 0.28);
  playTone(c, 659.25, c.currentTime + 0.16, 0.42);
}

function playTone(c: AudioContext, freq: number, startAt: number, durationSec: number) {
  const osc = c.createOscillator();
  const gain = c.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(freq, startAt);
  gain.gain.setValueAtTime(0.0001, startAt);
  gain.gain.exponentialRampToValueAtTime(0.25, startAt + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, startAt + durationSec);
  osc.connect(gain).connect(c.destination);
  osc.start(startAt);
  osc.stop(startAt + durationSec + 0.05);
}

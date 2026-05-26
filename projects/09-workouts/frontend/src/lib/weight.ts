// Weight is stored as i64 MINOR UNITS — grams. 1 kg = 1_000 g. This avoids
// every floating-point trap (75.1 kg is NOT 75.10000000000001 in JSON).
//
// Display rounds to one decimal kg. Parsing accepts integers ("80") and
// decimals ("82.5"). Volume is `weight_minor * reps` and stays in minor
// units throughout.

export function formatWeight(minor: number): string {
  if (!Number.isFinite(minor)) return '—';
  const kg = minor / 1000;
  // tabular nums look better in lists; we keep one decimal for clarity.
  return `${kg.toFixed(1)} kg`;
}

export function formatVolume(minor: number): string {
  if (!Number.isFinite(minor) || minor === 0) return '0 kg·rep';
  const kg = minor / 1000;
  if (kg >= 1000) {
    return `${(kg / 1000).toFixed(2)}t·rep`;
  }
  return `${kg.toFixed(0)} kg·rep`;
}

export function parseWeightKg(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const n = Number(trimmed);
  if (!Number.isFinite(n) || n <= 0) return null;
  return Math.round(n * 1000);
}

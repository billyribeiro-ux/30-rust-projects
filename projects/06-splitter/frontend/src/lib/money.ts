/**
 * Money is stored as `i64` cents on the backend and `number` cents on the
 * client. Float dollar amounts are NEVER kept in state — they're only
 * generated for display, and parsed back to integer cents on input.
 *
 * Why? `0.1 + 0.2 !== 0.3` in IEEE-754. Multiplying or summing dollar
 * amounts loses cents. With integer cents, every operation is exact.
 */

/** Format cents as a localized dollar string. `12345` → `"$123.45"`. */
export function formatMoney(cents: number, currency = 'USD'): string {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  }).format(cents / 100);
}

/** Format cents without a currency symbol. `12345` → `"123.45"`. */
export function formatCents(cents: number): string {
  const sign = cents < 0 ? '-' : '';
  const abs = Math.abs(cents);
  const dollars = Math.floor(abs / 100);
  const remainder = abs % 100;
  return `${sign}${dollars}.${String(remainder).padStart(2, '0')}`;
}

/**
 * Parse a user-typed dollar string into integer cents.
 * Accepts "12", "12.5", "12.34", " $12.34 ", "12,345.67".
 * Returns null on garbage.
 *
 * IMPORTANT: this is the ONLY place client-side that converts string→cents.
 * Centralizing the parse means there is one place to audit for float bugs.
 */
export function parseMoney(input: string): number | null {
  // Strip currency symbol, whitespace, thousand-separator commas
  const cleaned = input.replace(/[\s,$]/g, '').replace(/[^\d.\-]/g, '');
  if (cleaned === '' || cleaned === '-' || cleaned === '.') return null;

  const negative = cleaned.startsWith('-');
  const positive = negative ? cleaned.slice(1) : cleaned;

  if (positive.includes('.')) {
    const parts = positive.split('.');
    if (parts.length !== 2) return null;
    const dollars = parts[0] ?? '';
    const rawCents = parts[1] ?? '';
    if (rawCents.length > 2) return null;
    const cents = rawCents.padEnd(2, '0');
    const dollarsNum = dollars === '' ? 0 : Number(dollars);
    const centsNum = Number(cents);
    if (!Number.isFinite(dollarsNum) || !Number.isFinite(centsNum)) return null;
    if (!Number.isInteger(dollarsNum) || !Number.isInteger(centsNum)) return null;
    const total = dollarsNum * 100 + centsNum;
    return negative ? -total : total;
  }

  const dollars = Number(positive);
  if (!Number.isFinite(dollars) || !Number.isInteger(dollars)) return null;
  return negative ? -dollars * 100 : dollars * 100;
}

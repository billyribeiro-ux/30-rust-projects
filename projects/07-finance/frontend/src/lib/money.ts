/** See project 06: money is i64 cents (minor units) everywhere. */

export function formatMoney(minor: number, currency = 'USD'): string {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    signDisplay: 'auto'
  }).format(minor / 100);
}

export function formatCents(minor: number): string {
  const sign = minor < 0 ? '-' : '';
  const abs = Math.abs(minor);
  const dollars = Math.floor(abs / 100);
  const remainder = abs % 100;
  return `${sign}${dollars}.${String(remainder).padStart(2, '0')}`;
}

export function parseMoney(input: string): number | null {
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
    if (!Number.isInteger(dollarsNum) || !Number.isInteger(centsNum)) return null;
    const total = dollarsNum * 100 + centsNum;
    return negative ? -total : total;
  }
  const dollars = Number(positive);
  if (!Number.isInteger(dollars)) return null;
  return negative ? -dollars * 100 : dollars * 100;
}

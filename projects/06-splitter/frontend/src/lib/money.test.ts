import { describe, it, expect } from 'vitest';
import { parseMoney, formatCents } from './money';

describe('parseMoney', () => {
  it('parses whole dollars', () => {
    expect(parseMoney('12')).toBe(1200);
    expect(parseMoney('0')).toBe(0);
  });

  it('parses cents', () => {
    expect(parseMoney('12.34')).toBe(1234);
    expect(parseMoney('0.05')).toBe(5);
  });

  it('pads single-digit cents', () => {
    expect(parseMoney('12.5')).toBe(1250);
  });

  it('handles currency symbols and whitespace', () => {
    expect(parseMoney(' $12.34 ')).toBe(1234);
    expect(parseMoney('$1,234.56')).toBe(123456);
  });

  it('handles negatives', () => {
    expect(parseMoney('-12.34')).toBe(-1234);
  });

  it('rejects three-decimal-place input (would lose precision)', () => {
    expect(parseMoney('12.345')).toBe(null);
  });

  it('rejects garbage', () => {
    expect(parseMoney('hello')).toBe(null);
    expect(parseMoney('')).toBe(null);
    expect(parseMoney('.')).toBe(null);
    expect(parseMoney('1.2.3')).toBe(null);
  });
});

describe('formatCents', () => {
  it('formats positive cents', () => {
    expect(formatCents(1234)).toBe('12.34');
    expect(formatCents(5)).toBe('0.05');
    expect(formatCents(100)).toBe('1.00');
  });

  it('formats negative cents', () => {
    expect(formatCents(-1234)).toBe('-12.34');
  });

  it('formats zero', () => {
    expect(formatCents(0)).toBe('0.00');
  });
});

import { describe, it, expect } from 'vitest';
import { formatWeight, formatVolume, parseWeightKg } from './weight';

describe('formatWeight', () => {
  it('renders kg with one decimal', () => {
    expect(formatWeight(80_000)).toBe('80.0 kg');
    expect(formatWeight(82_500)).toBe('82.5 kg');
  });
  it('handles non-finite', () => {
    expect(formatWeight(Number.NaN)).toBe('—');
  });
});

describe('parseWeightKg', () => {
  it('parses integer kg into grams', () => {
    expect(parseWeightKg('80')).toBe(80_000);
  });
  it('parses decimal kg', () => {
    expect(parseWeightKg('82.5')).toBe(82_500);
  });
  it('rejects empty and non-positive', () => {
    expect(parseWeightKg('')).toBeNull();
    expect(parseWeightKg('0')).toBeNull();
    expect(parseWeightKg('-5')).toBeNull();
    expect(parseWeightKg('abc')).toBeNull();
  });
});

describe('formatVolume', () => {
  it('renders 0 as 0', () => {
    expect(formatVolume(0)).toBe('0 kg·rep');
  });
  it('renders small volumes in kg·rep', () => {
    expect(formatVolume(80_000 * 8)).toBe('640 kg·rep');
  });
  it('switches to tonnes·rep above 1000 kg·rep', () => {
    expect(formatVolume(2_000_000)).toBe('2.00t·rep');
  });
});

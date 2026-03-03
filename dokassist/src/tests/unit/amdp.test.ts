import { describe, it, expect } from 'vitest';
import { serializeAMDP, deserializeAMDP, AMDP_CATEGORIES } from '$lib/amdp';

describe('AMDP_CATEGORIES', () => {
  it('contains 12 categories', () => {
    expect(AMDP_CATEGORIES).toHaveLength(12);
  });

  it('every item has a code, label, and score of 0', () => {
    for (const category of AMDP_CATEGORIES) {
      for (const item of category.items) {
        expect(item).toHaveProperty('code');
        expect(item).toHaveProperty('label');
        expect(item.score).toBe(0);
      }
    }
  });
});

describe('serializeAMDP', () => {
  it('returns a JSON string', () => {
    const result = serializeAMDP(AMDP_CATEGORIES);
    expect(typeof result).toBe('string');
  });

  it('round-trips correctly with JSON.parse', () => {
    const result = serializeAMDP(AMDP_CATEGORIES);
    const parsed = JSON.parse(result);
    expect(parsed).toHaveLength(AMDP_CATEGORIES.length);
    expect(parsed[0].name).toBe(AMDP_CATEGORIES[0].name);
  });

  it('preserves item scores', () => {
    const modified = JSON.parse(JSON.stringify(AMDP_CATEGORIES));
    modified[0].items[0].score = 2;
    const serialized = serializeAMDP(modified);
    const parsed = JSON.parse(serialized);
    expect(parsed[0].items[0].score).toBe(2);
  });
});

describe('deserializeAMDP', () => {
  it('returns a deep clone of AMDP_CATEGORIES for null input', () => {
    const result = deserializeAMDP(null);
    expect(result).toHaveLength(AMDP_CATEGORIES.length);
    expect(result).not.toBe(AMDP_CATEGORIES); // different reference
    expect(result[0].items[0].score).toBe(0);
  });

  it('returns a deep clone of AMDP_CATEGORIES for invalid JSON', () => {
    const result = deserializeAMDP('not valid json {{');
    expect(result).toHaveLength(AMDP_CATEGORIES.length);
  });

  it('returns a deep clone of AMDP_CATEGORIES for empty string', () => {
    const result = deserializeAMDP('');
    expect(result).toHaveLength(AMDP_CATEGORIES.length);
  });

  it('correctly deserializes valid serialized data', () => {
    const serialized = serializeAMDP(AMDP_CATEGORIES);
    const result = deserializeAMDP(serialized);
    expect(result).toEqual(AMDP_CATEGORIES);
  });

  it('preserves modified scores after a round-trip', () => {
    const modified = JSON.parse(JSON.stringify(AMDP_CATEGORIES)) as typeof AMDP_CATEGORIES;
    modified[2].items[0].score = 3;
    const serialized = serializeAMDP(modified);
    const result = deserializeAMDP(serialized);
    expect(result[2].items[0].score).toBe(3);
  });
});

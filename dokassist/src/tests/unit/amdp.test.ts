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

  it('returns the parsed value as-is when JSON is valid but not an array', () => {
    // The function does not validate the shape of the parsed value;
    // it returns JSON.parse output directly for valid JSON.
    const result = deserializeAMDP('{"not": "an array"}') as unknown;
    expect(result).toEqual({ not: 'an array' });
  });
});

// ---------------------------------------------------------------------------
// serializeAMDP — additional edge cases
// ---------------------------------------------------------------------------

describe('serializeAMDP — edge cases', () => {
  it('serializes an empty categories array to "[]"', () => {
    expect(serializeAMDP([])).toBe('[]');
  });

  it('serializes and preserves all 12 category names', () => {
    const parsed = JSON.parse(serializeAMDP(AMDP_CATEGORIES));
    const names = parsed.map((c: { name: string }) => c.name);
    const expected = AMDP_CATEGORIES.map((c) => c.name);
    expect(names).toEqual(expected);
  });
});

// ---------------------------------------------------------------------------
// deserializeAMDP — additional edge cases
// ---------------------------------------------------------------------------

describe('deserializeAMDP — additional edge cases', () => {
  it('returns default categories for undefined (treated as null)', () => {
    // undefined coerces to null in the function's falsy check
    const result = deserializeAMDP(undefined as unknown as null);
    expect(result).toHaveLength(AMDP_CATEGORIES.length);
  });

  it('returns a fresh deep clone each time null is passed', () => {
    const first = deserializeAMDP(null);
    const second = deserializeAMDP(null);
    expect(first).not.toBe(second);
    first[0].items[0].score = 3;
    expect(second[0].items[0].score).toBe(0);
  });

  it('returns a fresh deep clone each time invalid JSON is passed', () => {
    const first = deserializeAMDP('{{invalid}}');
    const second = deserializeAMDP('{{invalid}}');
    expect(first).not.toBe(second);
    first[0].items[0].score = 2;
    expect(second[0].items[0].score).toBe(0);
  });
});

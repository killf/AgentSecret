import { expect, test } from 'vitest';
import { sha1 } from './sha1';

test('calculates SHA-1 hex digest', async () => {
  expect(await sha1('abc')).toBe('a9993e364706816aba3e25717850c26c9cd0d89d');
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { hasMeaningfulHistory, historicalPoint, isFresh } from '../src/domain/market.ts';

test('freshness boundary is explicit', () => {
  assert.equal(isFresh(0), true);
  assert.equal(isFresh(45_000), true);
  assert.equal(isFresh(45_001), false);
  assert.equal(isFresh(-1), false);
});

test('historicalPoint prefers the one-hour realized horizon', () => {
  const point = historicalPoint([
    { horizon: '15m', winRate: 48, samples: 100 },
    { horizon: '1h', winRate: 56, samples: 90 },
  ]);
  assert.equal(point?.horizon, '1h');
  assert.equal(point?.winRate, 56);
});

test('historical probability requires a meaningful sample size', () => {
  assert.equal(hasMeaningfulHistory({ horizon: '1h', winRate: 70, samples: 29 }), false);
  assert.equal(hasMeaningfulHistory({ horizon: '1h', winRate: 55, samples: 30 }), true);
});

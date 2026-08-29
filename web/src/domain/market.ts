export interface HistoricalPerformancePoint {
  horizon: string;
  winRate: number;
  samples: number;
}

export function isFresh(freshnessMs: number, maxAgeMs = 45_000): boolean {
  return Number.isFinite(freshnessMs) && freshnessMs >= 0 && freshnessMs <= maxAgeMs;
}

export function historicalPoint(
  points: HistoricalPerformancePoint[] | null | undefined,
  preferredHorizon = '1h',
): HistoricalPerformancePoint | undefined {
  if (!points?.length) return undefined;
  return points.find((point) => point.horizon.toLowerCase() === preferredHorizon.toLowerCase()) ?? points[0];
}

export function hasMeaningfulHistory(point: HistoricalPerformancePoint | undefined, minimumSamples = 30): boolean {
  return Boolean(point && point.samples >= minimumSamples && Number.isFinite(point.winRate));
}

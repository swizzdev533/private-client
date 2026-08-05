type Bucket = {
  count: number;
  resetAt: number;
};

const buckets = new Map<string, Bucket>();

export function checkRateLimit(
  key: string,
  limitPerMinute: number,
  nowMs: number = Date.now(),
): boolean {
  const windowMs = 60_000;
  const existing = buckets.get(key);
  if (!existing || existing.resetAt <= nowMs) {
    buckets.set(key, { count: 1, resetAt: nowMs + windowMs });
    return true;
  }
  if (existing.count >= limitPerMinute) {
    return false;
  }
  existing.count += 1;
  return true;
}

/** Test helper */
export function resetRateLimits(): void {
  buckets.clear();
}

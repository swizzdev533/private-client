export function presenceTtlSeconds(): number {
  const raw = Number(process.env.PRESENCE_TTL_SECONDS ?? "45");
  if (!Number.isFinite(raw) || raw < 15 || raw > 300) {
    return 45;
  }
  return Math.floor(raw);
}

export function rateLimitPerMinute(): number {
  const raw = Number(process.env.RATE_LIMIT_PER_MINUTE ?? "30");
  if (!Number.isFinite(raw) || raw < 5 || raw > 120) {
    return 30;
  }
  return Math.floor(raw);
}

export function assertBlobConfigured(): void {
  if (!process.env.BLOB_READ_WRITE_TOKEN) {
    throw new Error("BLOB_READ_WRITE_TOKEN is required");
  }
}

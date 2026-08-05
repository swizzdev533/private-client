import { NextResponse } from "next/server";
import { rateLimitPerMinute } from "@/lib/db";
import { upsertAndListPeers } from "@/lib/presence";
import { checkRateLimit } from "@/lib/rateLimit";
import { presenceRequestSchema } from "@/lib/validate";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

function clientIp(request: Request): string {
  const forwarded = request.headers.get("x-forwarded-for");
  if (forwarded) {
    return forwarded.split(",")[0]?.trim() || "unknown";
  }
  return request.headers.get("x-real-ip") ?? "unknown";
}

export async function POST(request: Request): Promise<Response> {
  let json: unknown;
  try {
    json = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const parsed = presenceRequestSchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json(
      { error: "Invalid presence payload", details: parsed.error.flatten() },
      { status: 400 },
    );
  }

  const ip = clientIp(request);
  const limit = rateLimitPerMinute();
  if (
    !checkRateLimit(`ip:${ip}`, limit) ||
    !checkRateLimit(`uuid:${parsed.data.uuid}`, limit)
  ) {
    return NextResponse.json({ error: "Rate limit exceeded" }, { status: 429 });
  }

  try {
    const response = await upsertAndListPeers(parsed.data);
    return NextResponse.json(response, {
      status: 200,
      headers: {
        "cache-control": "no-store",
      },
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Presence failed";
    if (message.includes("BLOB_READ_WRITE_TOKEN")) {
      return NextResponse.json({ error: "Presence store unavailable" }, { status: 503 });
    }
    return NextResponse.json({ error: "Presence store unavailable" }, { status: 503 });
  }
}

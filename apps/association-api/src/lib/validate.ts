import { z } from "zod";

export const PRESENCE_SCHEMA_VERSION = 1;

export const presenceRequestSchema = z.object({
  schemaVersion: z.literal(PRESENCE_SCHEMA_VERSION),
  uuid: z
    .string()
    .uuid()
    .transform((value) => value.toLowerCase()),
  username: z
    .string()
    .min(1)
    .max(16)
    .regex(/^[A-Za-z0-9_]{1,16}$/),
  serverHash: z.string().regex(/^[0-9a-f]{64}$/),
  clientVersion: z.string().min(1).max(32),
});

export type PresenceRequest = z.infer<typeof presenceRequestSchema>;

export const presenceResponseSchema = z.object({
  schemaVersion: z.literal(PRESENCE_SCHEMA_VERSION),
  peers: z.array(z.string().uuid()),
  peerEntries: z
    .array(
      z.object({
        uuid: z.string().uuid(),
        username: z
          .string()
          .min(1)
          .max(16)
          .regex(/^[A-Za-z0-9_]{1,16}$/)
          .optional(),
      }),
    )
    .optional(),
});

export type PresenceResponse = z.infer<typeof presenceResponseSchema>;
export type PresencePeer = {
  uuid: string;
  username?: string;
};

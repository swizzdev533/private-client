import { z } from "zod";

export const isoDateTimeSchema = z
  .string()
  .datetime({ offset: true })
  .or(
    z
      .string()
      .refine((val) => !isNaN(Date.parse(val)), { message: "Invalid ISO date string" }),
  );

export const launchStateSchema = z.enum([
  "IDLE",
  "VALIDATING",
  "CHECKING_RUNTIME",
  "PREPARING_INSTANCE",
  "VERIFYING_GAME_FILES",
  "INSTALLING_GAME_FILES",
  "VERIFYING_FORGE",
  "INSTALLING_FORGE",
  "CHECKING_REQUIRED_MODS",
  "APPLYING_PENDING_CHANGES",
  "BUILDING_LAUNCH_COMMAND",
  "LAUNCHING",
  "RUNNING",
  "STOPPING",
  "EXITED",
  "FAILED",
]);

export type LaunchState = z.infer<typeof launchStateSchema>;

export const launchProgressSchema = z.object({
  state: launchStateSchema,
  message: z.string().min(1),
  progress: z.number().min(0).max(100).nullable(),
  canCancel: z.boolean(),
  errorId: z.string().nullable(),
  logPath: z.string().nullable(),
});

export type LaunchProgress = z.infer<typeof launchProgressSchema>;

export const playerProfileSchema = z.object({
  schemaVersion: z.number().int().positive(),
  username: z.string().min(1).max(16),
  uuid: z.string().min(32).max(36),
  skinModel: z.enum(["classic", "slim"]),
  skinPath: z.string().nullable(),
  updatedAt: isoDateTimeSchema,
});

export type PlayerProfile = z.infer<typeof playerProfileSchema>;
export const profileUpdatedEventSchema = playerProfileSchema.nullable();
export type ProfileUpdatedEvent = z.infer<typeof profileUpdatedEventSchema>;

export const launcherSettingsSchema = z
  .object({
    schemaVersion: z.number().int().positive(),
    javaPath: z.string().nullable(),
    memoryMinMb: z.number().int().min(512),
    memoryMaxMb: z.number().int().min(1024),
    reducedMotion: z.boolean(),
    autoUpdateChecks: z.boolean(),
    downloadConcurrency: z.number().int().min(1).max(8),
  })
  .refine((value) => value.memoryMaxMb >= value.memoryMinMb, {
    message: "Maximum RAM cannot be lower than the minimum.",
    path: ["memoryMaxMb"],
  });

export type LauncherSettings = z.infer<typeof launcherSettingsSchema>;

/** Result of a signed launcher update check. Verification happens in Rust. */
export const updateStatusSchema = z.object({
  available: z.boolean(),
  currentVersion: z.string().min(1),
  availableVersion: z.string().min(1).nullable(),
  // Release notes come from the update manifest: bounded and rendered as text.
  notes: z.string().max(2000).nullable(),
  publishedAt: z.string().nullable(),
});

export type UpdateStatus = z.infer<typeof updateStatusSchema>;

export const instanceSummarySchema = z.object({
  installed: z.boolean(),
  healthy: z.boolean(),
  minecraftVersion: z.literal("1.8.9"),
  forgeVersion: z.string(),
  javaLabel: z.string().nullable(),
  pendingOperations: z.number().int().nonnegative(),
  lastPlayedAt: isoDateTimeSchema.nullable(),
});

export type InstanceSummary = z.infer<typeof instanceSummarySchema>;

export const launcherSnapshotSchema = z.object({
  profile: playerProfileSchema.nullable(),
  launch: launchProgressSchema,
  settings: launcherSettingsSchema,
  instance: instanceSummarySchema,
  appVersion: z.string(),
  channel: z.enum(["stable", "beta"]),
});

export type LauncherSnapshot = z.infer<typeof launcherSnapshotSchema>;

export const modTrustSchema = z.enum(["VERIFIED", "FROM_MODRINTH"]);
export type ModTrust = z.infer<typeof modTrustSchema>;

export const modCompatibilitySchema = z.enum([
  "COMPATIBLE",
  "EXPERIMENTAL",
  "LICENSE_REVIEW",
  "INCOMPATIBLE",
  "DOWNLOAD_UNAVAILABLE",
]);

export type ModCompatibility = z.infer<typeof modCompatibilitySchema>;

/**
 * Provider icons are decorative and arrive unvalidated from Modrinth, which
 * returns "" for iconless projects. Rejecting those would discard the whole
 * result page, so an unusable value degrades to null instead. The https check
 * is the actual trust-boundary control: zod's .url() accepts `javascript:` and
 * `file://`, and this value is rendered directly into an <img src>.
 */
const providerIconUrlSchema = z
  .string()
  .nullable()
  .transform((value) => (value !== null && /^https:\/\/\S+$/.test(value) ? value : null));

export const modSummarySchema = z.object({
  id: z.string().min(1),
  projectId: z.string().min(1),
  versionId: z.string().min(1),
  name: z.string(),
  author: z.string(),
  description: z.string(),
  iconUrl: providerIconUrlSchema,
  version: z.string().min(1),
  releaseType: z.enum(["release", "beta", "alpha"]),
  downloads: z.number().int().nonnegative(),
  updatedAt: isoDateTimeSchema,
  minecraftVersion: z.literal("1.8.9"),
  loader: z.literal("forge"),
  environment: z.enum(["client", "client_and_server"]),
  license: z.string(),
  fileSize: z.number().int().nonnegative(),
  dependencyCount: z.number().int().nonnegative(),
  trust: modTrustSchema,
  compatibility: modCompatibilitySchema,
  compatibilityReason: z.string().nullable(),
  installed: z.boolean(),
  installedVersion: z.string().nullable(),
  updateAvailable: z.boolean(),
  required: z.boolean(),
});

export type ModSummary = z.infer<typeof modSummarySchema>;

export const installedModSchema = modSummarySchema.extend({
  fileName: z.string().min(1),
  // The backend emits "" when a record predates hash tracking; that must not
  // erase the entire installed list.
  sha512: z.string(),
  installedAt: isoDateTimeSchema,
  dependencies: z.array(z.string()),
  dependents: z.array(z.string()),
  provider: z.enum(["modrinth", "local-import", "private-client", "github"]),
});

export type InstalledMod = z.infer<typeof installedModSchema>;

export const modSearchRequestSchema = z.object({
  query: z.string().max(120),
  sort: z.enum(["relevance", "downloads", "updated"]),
  trust: z.enum(["all", "verified", "modrinth"]),
  page: z.number().int().nonnegative(),
});

export type ModSearchRequest = z.infer<typeof modSearchRequestSchema>;

/**
 * Drops individual malformed rows rather than failing the whole page, so one
 * bad upstream record degrades the result set instead of blanking the view.
 */
function resilientArray<Schema extends z.ZodTypeAny>(schema: Schema) {
  return z.array(z.unknown()).transform((rows) =>
    rows.reduce<z.output<Schema>[]>((accepted, row) => {
      const parsed = schema.safeParse(row);
      if (parsed.success) {
        accepted.push(parsed.data);
      }
      return accepted;
    }, []),
  );
}

export const modSearchResponseSchema = z.object({
  query: z.string(),
  results: resilientArray(modSummarySchema),
  page: z.number().int().nonnegative(),
  hasMore: z.boolean(),
  fromCache: z.boolean(),
  offline: z.boolean(),
});

export type ModSearchResponse = z.infer<typeof modSearchResponseSchema>;

export const installPlanItemSchema = z.object({
  projectId: z.string(),
  versionId: z.string(),
  name: z.string(),
  version: z.string(),
  fileSize: z.number().int().nonnegative(),
  required: z.boolean(),
});

export const installPlanSchema = z.object({
  requestedMod: installPlanItemSchema,
  dependencies: z.array(installPlanItemSchema),
  expectedDiskUsage: z.number().int().nonnegative(),
  filesToReplace: z.array(z.string()),
  warnings: z.array(z.string()),
});

export type InstallPlan = z.infer<typeof installPlanSchema>;

export const pendingOperationSchema = z.object({
  id: z.string(),
  type: z.enum(["INSTALL", "REMOVE", "UPDATE", "IMPORT_OPTIFINE"]),
  targetId: z.string(),
  targetName: z.string(),
  createdAt: isoDateTimeSchema,
  status: z.enum(["PENDING", "RUNNING", "FAILED"]),
  retryCount: z.number().int().nonnegative(),
  errorMessage: z.string().nullable(),
});

export type PendingOperation = z.infer<typeof pendingOperationSchema>;

export const operationProgressSchema = z.object({
  operationId: z.string(),
  targetId: z.string(),
  phase: z.string(),
  message: z.string(),
  progress: z.number().min(0).max(100),
});

export type OperationProgress = z.infer<typeof operationProgressSchema>;

export const commandResultSchema = z.object({
  ok: z.literal(true),
  message: z.string(),
  queued: z.boolean(),
});

export type CommandResult = z.infer<typeof commandResultSchema>;

export interface DomainErrorShape {
  id: string;
  title: string;
  message: string;
  resolution: string | null;
  logPath: string | null;
}

export class DomainError extends Error {
  readonly id: string;
  readonly title: string;
  readonly resolution: string | null;
  readonly logPath: string | null;

  constructor(shape: DomainErrorShape) {
    super(shape.message);
    this.name = "DomainError";
    this.id = shape.id;
    this.title = shape.title;
    this.resolution = shape.resolution;
    this.logPath = shape.logPath;
  }
}

export const launchStateLabels: Readonly<Record<LaunchState, string>> = {
  IDLE: "Ready to play",
  VALIDATING: "Checking configuration",
  CHECKING_RUNTIME: "Checking Java 8",
  PREPARING_INSTANCE: "Preparing the instance",
  VERIFYING_GAME_FILES: "Verifying game files",
  INSTALLING_GAME_FILES: "Installing Minecraft 1.8.9",
  VERIFYING_FORGE: "Verifying Forge",
  INSTALLING_FORGE: "Installing Forge",
  CHECKING_REQUIRED_MODS: "Checking required mods",
  APPLYING_PENDING_CHANGES: "Applying pending changes",
  BUILDING_LAUNCH_COMMAND: "Preparing the process",
  LAUNCHING: "Starting the game",
  RUNNING: "The game is running",
  STOPPING: "Stopping the game",
  EXITED: "The game was closed",
  FAILED: "Startup failed",
};

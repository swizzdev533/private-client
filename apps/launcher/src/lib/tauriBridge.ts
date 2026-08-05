import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { z } from "zod";
import { DomainError, type DomainErrorShape } from "../types/contracts";

export const isTauriRuntime =
  typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;

async function loadDevelopmentBackend() {
  if (!import.meta.env.DEV) {
    throw new DomainError({
      id: "BackendUnavailable",
      title: "The backend is unavailable",
      message: "The production launcher requires the native Tauri runtime.",
      resolution: "Start the installed Private Client application.",
      logPath: null,
    });
  }
  return import("./demoBackend");
}

function errorShape(error: unknown): DomainErrorShape {
  if (error instanceof DomainError) {
    return {
      id: error.id,
      title: error.title,
      message: error.message,
      resolution: error.resolution,
      logPath: error.logPath,
    };
  }

  if (typeof error === "object" && error !== null) {
    const value = error as Record<string, unknown>;
    return {
      id: typeof value.id === "string" ? value.id : "UnexpectedError",
      title: typeof value.title === "string" ? value.title : "The operation failed",
      message:
        typeof value.message === "string"
          ? value.message
          : "The backend returned an unknown error.",
      resolution: typeof value.resolution === "string" ? value.resolution : null,
      logPath: typeof value.logPath === "string" ? value.logPath : null,
    };
  }

  return {
    id: "UnexpectedError",
    title: "The operation failed",
    message: error instanceof Error ? error.message : String(error),
    resolution: null,
    logPath: null,
  };
}

export async function invokeValidated<TSchema extends z.ZodType>(
  command: string,
  schema: TSchema,
  args?: Record<string, unknown>,
): Promise<z.infer<TSchema>> {
  try {
    const raw = isTauriRuntime
      ? await invoke<unknown>(command, args)
      : await loadDevelopmentBackend().then((backend) => backend.invokeDemo(command, args));
    return schema.parse(raw);
  } catch (error) {
    throw new DomainError(errorShape(error));
  }
}

export async function listenValidated<TSchema extends z.ZodType>(
  eventName:
    | "launcher://launch-state"
    | "launcher://profile-updated"
    | "launcher://mods-changed"
    | "launcher://operation-progress",
  schema: TSchema,
  handler: (payload: z.infer<TSchema>) => void,
): Promise<UnlistenFn> {
  if (!isTauriRuntime) {
    if (!import.meta.env.DEV) {
      return () => undefined;
    }
    const backend = await loadDevelopmentBackend();
    return backend.subscribeDemo(eventName, (payload) => {
      const result = schema.safeParse(payload);
      if (result.success) {
        handler(result.data);
      }
    });
  }

  return listen<unknown>(eventName, (event) => {
    const result = schema.safeParse(event.payload);
    if (result.success) {
      handler(result.data);
    }
  });
}

export function localAssetUrl(path: string | null): string | null {
  if (!path) {
    return null;
  }
  if (/^(?:https?:|data:|blob:)/u.test(path)) {
    return path;
  }
  return isTauriRuntime ? convertFileSrc(path) : null;
}

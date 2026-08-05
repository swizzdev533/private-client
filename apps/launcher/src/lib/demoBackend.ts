import { demoInstalled, demoLibrary, demoPending, demoSnapshot } from "./demoFixtures";
import type {
  InstallPlan,
  InstalledMod,
  LauncherSettings,
  LauncherSnapshot,
  ModSearchRequest,
  ModSearchResponse,
  ModSummary,
  OperationProgress,
  PendingOperation,
} from "../types/contracts";
import { DomainError } from "../types/contracts";

type DemoEventName =
  | "launcher://launch-state"
  | "launcher://profile-updated"
  | "launcher://mods-changed"
  | "launcher://operation-progress";

type DemoListener = (payload: unknown) => void;

const listeners = new Map<DemoEventName, Set<DemoListener>>();
let snapshot = structuredClone(demoSnapshot);
let library = structuredClone(demoLibrary);
let installed = structuredClone(demoInstalled);
let pending = structuredClone(demoPending);
let launchTimers: number[] = [];

function emit(name: DemoEventName, payload: unknown): void {
  listeners.get(name)?.forEach((listener) => {
    listener(structuredClone(payload));
  });
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value === "object" && value !== null) {
    return value as Record<string, unknown>;
  }
  return {};
}

function getString(args: unknown, key: string): string {
  const value = record(args)[key];
  if (typeof value !== "string") {
    throw new Error(`Brak poprawnego argumentu: ${key}`);
  }
  return value;
}

function clearLaunchTimers(): void {
  launchTimers.forEach((timer) => window.clearTimeout(timer));
  launchTimers = [];
}

function setLaunchState(
  state: LauncherSnapshot["launch"]["state"],
  message: string,
  progress: number | null,
  canCancel: boolean,
): void {
  snapshot.launch = {
    state,
    message,
    progress,
    canCancel,
    errorId: null,
    logPath: null,
  };
  emit("launcher://launch-state", snapshot.launch);
}

function startDemoLaunch(): void {
  clearLaunchTimers();
  const steps: Array<
    [LauncherSnapshot["launch"]["state"], string, number, boolean, number]
  > = [
    ["VALIDATING", "Sprawdzanie konfiguracji", 6, true, 0],
    ["CHECKING_RUNTIME", "Sprawdzanie Java 8", 14, true, 260],
    ["PREPARING_INSTANCE", "Przygotowywanie izolowanej instancji", 26, true, 520],
    ["VERIFYING_GAME_FILES", "Weryfikowanie plików gry", 42, true, 780],
    ["VERIFYING_FORGE", "Weryfikowanie Forge 1.8.9", 57, true, 1_040],
    ["CHECKING_REQUIRED_MODS", "Sprawdzanie wymaganych modów", 70, true, 1_300],
    ["APPLYING_PENDING_CHANGES", "Stosowanie oczekujących zmian", 80, false, 1_560],
    ["BUILDING_LAUNCH_COMMAND", "Przygotowywanie bezpiecznego procesu", 90, false, 1_820],
    ["LAUNCHING", "Uruchamianie Minecraft", 97, false, 2_080],
    ["RUNNING", "Minecraft 1.8.9 działa", 100, false, 2_440],
  ];

  steps.forEach(([state, message, progress, canCancel, delay]) => {
    launchTimers.push(
      window.setTimeout(() => {
        setLaunchState(state, message, progress, canCancel);
      }, delay),
    );
  });

  if (new URLSearchParams(window.location.search).get("e2eCrash") === "1") {
    launchTimers.push(
      window.setTimeout(() => {
        snapshot.launch = {
          state: "FAILED",
          message: "Kontrolowany crash Forge z fixture E2E",
          progress: null,
          canCancel: false,
          errorId: "GameCrashed",
          logPath: "C:\\PrivateClient\\logs\\fixture-crash.log",
        };
        emit("launcher://launch-state", snapshot.launch);
      }, 3_100),
    );
  }
}

function findLibraryMod(projectId: string): ModSummary {
  const selected = library.find((mod) => mod.projectId === projectId);
  if (!selected) {
    throw new Error("Mod nie istnieje w katalogu demonstracyjnym.");
  }
  return selected;
}

function toInstalled(mod: ModSummary): InstalledMod {
  return {
    ...mod,
    installed: true,
    installedVersion: mod.version,
    updateAvailable: false,
    fileName: `${mod.id}-${mod.version}.jar`,
    sha512: `demo-${mod.projectId}-sha512-000000000000000000000000`,
    installedAt: new Date().toISOString(),
    dependencies: [],
    dependents: [],
    provider: "modrinth",
  };
}

function syncLibraryInstallState(): void {
  library = library.map((candidate) => {
    const local = installed.find((item) => item.projectId === candidate.projectId);
    return {
      ...candidate,
      installed: local !== undefined,
      installedVersion: local?.installedVersion ?? null,
      updateAvailable: local !== undefined && local.installedVersion !== candidate.version,
    };
  });
}

function search(args: ModSearchRequest): ModSearchResponse {
  const normalized = args.query.trim().toLocaleLowerCase("pl-PL");
  let results = library.filter(
    (mod) =>
      normalized.length === 0 ||
      `${mod.name} ${mod.author} ${mod.description}`
        .toLocaleLowerCase("pl-PL")
        .includes(normalized),
  );

  if (args.trust === "verified") {
    results = results.filter((mod) => mod.trust === "VERIFIED");
  } else if (args.trust === "modrinth") {
    results = results.filter((mod) => mod.trust === "FROM_MODRINTH");
  }

  results.sort((left, right) => {
    if (args.sort === "downloads") {
      return right.downloads - left.downloads;
    }
    if (args.sort === "updated") {
      return Date.parse(right.updatedAt) - Date.parse(left.updatedAt);
    }
    return Number(right.trust === "VERIFIED") - Number(left.trust === "VERIFIED");
  });

  const pageSize = 20;
  const start = args.page * pageSize;
  const pagedResults = results.slice(start, start + pageSize);

  return {
    query: args.query,
    results: pagedResults,
    page: args.page,
    hasMore: start + pageSize < results.length,
    fromCache: false,
    offline: false,
  };
}

function installPlan(projectId: string): InstallPlan {
  const mod = findLibraryMod(projectId);
  const dependency =
    mod.dependencyCount > 0
      ? [
          {
            projectId: "demo-essential-library",
            versionId: "essential-library-1.2.0",
            name: "Essential Library",
            version: "1.2.0",
            fileSize: 483_120,
            required: true,
          },
        ]
      : [];

  return {
    requestedMod: {
      projectId: mod.projectId,
      versionId: mod.versionId,
      name: mod.name,
      version: mod.version,
      fileSize: mod.fileSize,
      required: false,
    },
    dependencies: dependency,
    expectedDiskUsage:
      mod.fileSize + dependency.reduce((sum, item) => sum + item.fileSize, 0),
    filesToReplace: mod.installed ? [`${mod.id}-old.jar`] : [],
    warnings:
      mod.releaseType === "beta"
        ? ["To wydanie beta. Może być mniej stabilne niż release."]
        : [],
  };
}

async function performInstall(projectId: string): Promise<{
  ok: true;
  message: string;
  queued: boolean;
}> {
  const mod = findLibraryMod(projectId);
  if (snapshot.launch.state === "RUNNING") {
    const operation: PendingOperation = {
      id: `pending-${Date.now()}`,
      type: "INSTALL",
      targetId: projectId,
      targetName: mod.name,
      createdAt: new Date().toISOString(),
      status: "PENDING",
      retryCount: 0,
      errorMessage: null,
    };
    pending = [...pending, operation];
    snapshot.instance.pendingOperations = pending.length;
    emit("launcher://mods-changed", { reason: "queued" });
    return {
      ok: true,
      message: "Instalacja została dodana do kolejki.",
      queued: true,
    };
  }

  const operationId = `install-${Date.now()}`;
  const phases: Array<[string, string, number]> = [
    ["RESOLVING_DEPENDENCIES", "Rozwiązywanie zależności", 18],
    ["DOWNLOADING_TEMPORARY_FILES", "Pobieranie do stagingu", 44],
    ["CALCULATING_SHA512", "Weryfikowanie SHA-512", 67],
    ["VALIDATING_JAR_STRUCTURE", "Walidowanie struktury JAR", 82],
    ["INSTALLING_ATOMICALLY", "Instalowanie atomowe", 94],
  ];

  for (const [phase, message, progress] of phases) {
    const payload: OperationProgress = {
      operationId,
      targetId: projectId,
      phase,
      message,
      progress,
    };
    emit("launcher://operation-progress", payload);
    await new Promise((resolve) => window.setTimeout(resolve, 110));
    if (projectId === "demo-download-error" && phase === "DOWNLOADING_TEMPORARY_FILES") {
      throw new DomainError({
        id: "DownloadFailed",
        title: "Pobieranie nie powiodło się",
        message: "Kontrolowany błąd pobierania z fixture E2E.",
        resolution: "Spróbuj ponownie po sprawdzeniu połączenia.",
        logPath: "C:\\PrivateClient\\logs\\fixture-download.log",
      });
    }
  }

  installed = [
    ...installed.filter((item) => item.projectId !== projectId),
    toInstalled(mod),
  ];
  syncLibraryInstallState();
  emit("launcher://operation-progress", {
    operationId,
    targetId: projectId,
    phase: "INSTALLED",
    message: "Mod został bezpiecznie zainstalowany",
    progress: 100,
  } satisfies OperationProgress);
  emit("launcher://mods-changed", { reason: "installed", projectId });

  return { ok: true, message: `${mod.name} został zainstalowany.`, queued: false };
}

export function subscribeDemo(name: DemoEventName, listener: DemoListener): () => void {
  const eventListeners = listeners.get(name) ?? new Set<DemoListener>();
  eventListeners.add(listener);
  listeners.set(name, eventListeners);

  return () => {
    eventListeners.delete(listener);
  };
}

export async function invokeDemo(command: string, args?: unknown): Promise<unknown> {
  switch (command) {
    case "get_launcher_snapshot":
      return structuredClone(snapshot);
    case "launch_game":
      if (snapshot.launch.state === "RUNNING") {
        return {
          ok: true,
          message: "Okno gry zostało przywrócone.",
          queued: false,
        };
      }
      startDemoLaunch();
      return { ok: true, message: "Uruchamianie rozpoczęte.", queued: false };
    case "cancel_launch":
      clearLaunchTimers();
      setLaunchState("IDLE", "Uruchamianie anulowane bezpiecznie", null, false);
      return { ok: true, message: "Operacja została anulowana.", queued: false };
    case "stop_game":
      setLaunchState("STOPPING", "Zamykanie procesu gry", null, false);
      window.setTimeout(() => {
        setLaunchState("EXITED", "Gra została zamknięta poprawnie", null, false);
      }, 500);
      return { ok: true, message: "Wysłano prośbę o zamknięcie.", queued: false };
    case "focus_game_window":
      return { ok: true, message: "Okno gry zostało przywrócone.", queued: false };
    case "save_launcher_settings": {
      const settings = record(args).settings as LauncherSettings;
      snapshot.settings = structuredClone(settings);
      return structuredClone(snapshot.settings);
    }
    case "open_logs_directory":
      return { ok: true, message: "Otworzono katalog logów.", queued: false };
    case "export_logs":
      return {
        ok: true,
        message: "Utworzono zredagowane lokalne archiwum logów.",
        queued: false,
      };
    // The dev backend never reaches a real release host; it reports "current"
    // so the update UI stays reachable without faking an installable artifact.
    case "check_for_update":
      return {
        available: false,
        currentVersion: snapshot.appVersion,
        availableVersion: null,
        notes: null,
        publishedAt: null,
      };
    case "install_update":
      throw new DomainError({
        id: "UpdateFailed",
        title: "Aktualizacje są niedostępne w trybie deweloperskim",
        message: "Podpisany kanał aktualizacji działa tylko w zbudowanej aplikacji.",
        resolution: "Uruchom zainstalowaną aplikację Private Client.",
        logPath: null,
      });
    case "search_modrinth":
      return search(record(args).request as ModSearchRequest);
    case "get_mod_install_plan":
      return installPlan(getString(args, "projectId"));
    case "install_mod":
      return performInstall(getString(args, "projectId"));
    case "list_installed_mods":
      return structuredClone(installed);
    case "list_pending_operations":
      return structuredClone(pending);
    case "remove_mod": {
      const projectId = getString(args, "projectId");
      const selected = installed.find((mod) => mod.projectId === projectId);
      if (!selected) {
        throw new Error("Mod nie jest zainstalowany.");
      }
      if (selected.required || selected.dependents.length > 0) {
        throw new Error("Ten mod jest wymagany i nie może zostać usunięty.");
      }
      if (snapshot.launch.state === "RUNNING") {
        pending = [
          ...pending,
          {
            id: `pending-${Date.now()}`,
            type: "REMOVE",
            targetId: projectId,
            targetName: selected.name,
            createdAt: new Date().toISOString(),
            status: "PENDING",
            retryCount: 0,
            errorMessage: null,
          },
        ];
        snapshot.instance.pendingOperations = pending.length;
        emit("launcher://mods-changed", { reason: "queued" });
        return {
          ok: true,
          message: "Usuwanie zostało dodane do kolejki.",
          queued: true,
        };
      }
      installed = installed.filter((mod) => mod.projectId !== projectId);
      syncLibraryInstallState();
      emit("launcher://mods-changed", { reason: "removed", projectId });
      return { ok: true, message: `${selected.name} został usunięty.`, queued: false };
    }
    case "update_mod": {
      const projectId = getString(args, "projectId");
      return performInstall(projectId);
    }
    case "cancel_pending_operation": {
      const operationId = getString(args, "operationId");
      pending = pending.filter((operation) => operation.id !== operationId);
      snapshot.instance.pendingOperations = pending.length;
      emit("launcher://mods-changed", { reason: "queue-updated" });
      return { ok: true, message: "Usunięto operację z kolejki.", queued: false };
    }
    case "apply_pending_operations":
      pending = [];
      snapshot.instance.pendingOperations = 0;
      emit("launcher://mods-changed", { reason: "queue-applied" });
      return { ok: true, message: "Zastosowano oczekujące operacje.", queued: false };
    case "download_optifine":
    case "import_optifine": {
      const existingOptifine = installed.some((mod) => mod.id === "optifine-local");
      const existingHitDelay = installed.some((mod) => mod.id === "hitdelayfix-local");
      const existingAnimations = installed.some((mod) => mod.id === "animations-external");
      const existingFullbright = installed.some((mod) => mod.id === "fullbright-local");
      const template = demoLibrary[1]!;
      const newMods: InstalledMod[] = [];
      if (!existingOptifine) {
        newMods.push({
          ...template,
          id: "optifine-local",
          projectId: "local-optifine",
          versionId: "local-import",
          name: "OptiFine 1.8.9",
          author: "sp614x",
          description: "Oficjalne wydanie OptiFine HD U M5 dla 1.8.9.",
          version: "HD U M5",
          license: "External local file",
          trust: "VERIFIED",
          installed: true,
          installedVersion: "HD U M5",
          required: false,
          fileName: "OptiFine_1.8.9_HD_U_M5.jar",
          sha512: "demo-local-import-sha512-0000000000000000000000",
          installedAt: new Date().toISOString(),
          dependencies: [],
          dependents: [],
          provider: "local-import",
        });
      }
      if (!existingAnimations) {
        newMods.push({
          ...template,
          id: "animations-external",
          projectId: "4Hfmgaef",
          versionId: "x99qPdUO",
          name: "Animatium Legacy (OverflowAnimations)",
          author: "Polyfrost",
          description: "Oryginalny zewnętrzny mod animacji 1.7 dla Forge 1.8.9.",
          version: "2.2.2",
          license: "LGPL-3.0-only",
          trust: "FROM_MODRINTH",
          installed: true,
          installedVersion: "2.2.2",
          required: false,
          fileName: "OverflowAnimations-1.8.9-forge-2.2.2.jar",
          sha512:
            "b1167b5bd8207af1b95c755124d298d4c0ddc25e975da5e5eb9548d8e4336a00c878f75f999a1babd9e9cfc5b362352737651c266fcaa950815a513af42c0c5b",
          installedAt: new Date().toISOString(),
          dependencies: [],
          dependents: [],
          provider: "modrinth",
        });
      }
      if (!existingHitDelay) {
        newMods.push({
          ...template,
          id: "hitdelayfix-local",
          projectId: "local-hitdelayfix",
          versionId: "1.0.1",
          name: "HitDelayFix",
          author: "ghast",
          description:
            "Usunięcie opóźnienia ataku (leftClickCounter) po nietrafionym ciosie.",
          version: "1.0.1",
          license: "MIT",
          trust: "VERIFIED",
          installed: true,
          installedVersion: "1.0.1",
          required: false,
          fileName: "HitDelayFix-1.0.1.jar",
          sha512:
            "b8b49155b836caf4e9c9ba03f803900fa3e3d9d45f96fa8487b04cd37ceacbb8b8c8794348c49be0a5d7d52e0dde12265c18db4292a9421f76f1cdb0e16ca0c2",
          installedAt: new Date().toISOString(),
          dependencies: [],
          dependents: [],
          provider: "github",
        });
      }
      if (!existingFullbright) {
        newMods.push({
          ...template,
          id: "fullbright-local",
          projectId: "local-fullbright",
          versionId: "v1.0.0",
          name: "Fullbright",
          author: "Modrinth",
          description: "Maksymalna jasność w ciemnościach i jaskiniach bez cieni.",
          version: "1.0.0",
          license: "CC-BY-NC-ND-4.0",
          trust: "FROM_MODRINTH",
          installed: true,
          installedVersion: "1.0.0",
          required: false,
          fileName: "Fullbright-1.0.0.jar",
          sha512:
            "f9a54aeb27196958b75bb77d5025fa0c64f61eb605d86726354ba741d817769be9a5e287db578b103d2c7fb104d851fb16a3c46a378a3954ba05c6a48411ac25",
          installedAt: new Date().toISOString(),
          dependencies: [],
          dependents: [],
          provider: "modrinth",
        });
      }
      if (newMods.length > 0) {
        installed = [...installed, ...newMods];
      }
      emit("launcher://mods-changed", { reason: "optifine-imported" });
      return {
        ok: true,
        message:
          "Private Pack z oryginalnymi HitDelayFix, Animatium Legacy i Fullbright został zweryfikowany i zainstalowany.",
        queued: false,
      };
    }
    default:
      throw new Error(`Brak demonstracyjnej implementacji komendy: ${command}`);
  }
}

export function resetDemoBackend(): void {
  clearLaunchTimers();
  snapshot = structuredClone(demoSnapshot);
  library = structuredClone(demoLibrary);
  installed = structuredClone(demoInstalled);
  pending = structuredClone(demoPending);
  listeners.clear();
}

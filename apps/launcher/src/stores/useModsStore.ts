import { create } from "zustand";
import { modsApi } from "../services/modsApi";
import {
  DomainError,
  type InstallPlan,
  type InstalledMod,
  type ModSearchRequest,
  type ModSummary,
  type OperationProgress,
  type PendingOperation,
} from "../types/contracts";
import { useUiStore } from "./useUiStore";

export interface ModsState {
  query: string;
  sort: ModSearchRequest["sort"];
  trust: ModSearchRequest["trust"];
  page: number;
  hasMore: boolean;
  results: ModSummary[];
  installed: InstalledMod[];
  pending: PendingOperation[];
  selectedMod: ModSummary | null;
  installPlan: InstallPlan | null;
  operationProgress: OperationProgress | null;
  searching: boolean;
  refreshing: boolean;
  mutatingProjectId: string | null;
  setQuery: (query: string) => void;
  setSort: (sort: ModSearchRequest["sort"]) => void;
  setTrust: (trust: ModSearchRequest["trust"]) => void;
  setPage: (page: number) => void;
  nextPage: () => Promise<void>;
  prevPage: () => Promise<void>;
  search: (targetPage?: number) => Promise<void>;
  refreshLocalState: () => Promise<void>;
  prepareInstall: (mod: ModSummary) => Promise<void>;
  closeInstallPlan: () => void;
  confirmInstall: () => Promise<void>;
  remove: (mod: InstalledMod) => Promise<void>;
  update: (mod: InstalledMod) => Promise<void>;
  downloadOptifine: () => Promise<void>;
  importOptifine: () => Promise<void>;
  cancelPending: (operationId: string) => Promise<void>;
  applyPending: () => Promise<void>;
  applyOperationProgress: (progress: OperationProgress) => void;
}

let searchSequence = 0;
let planSequence = 0;

function notifyError(error: unknown): void {
  const domain =
    error instanceof DomainError
      ? error
      : new DomainError({
          id: "UnexpectedError",
          title: "Operacja nie powiodła się",
          message: error instanceof Error ? error.message : String(error),
          resolution: null,
          logPath: null,
        });
  useUiStore.getState().notify({
    tone: "error",
    title: `${domain.title} · ${domain.id}`,
    message: domain.resolution ? `${domain.message} ${domain.resolution}` : domain.message,
  });
}

export const useModsStore = create<ModsState>((set, get) => ({
  query: "",
  sort: "relevance",
  trust: "all",
  page: 0,
  hasMore: false,
  results: [],
  installed: [],
  pending: [],
  selectedMod: null,
  installPlan: null,
  operationProgress: null,
  searching: false,
  refreshing: false,
  mutatingProjectId: null,
  setQuery: (query) => {
    set({ query, page: 0 });
  },
  setSort: (sort) => {
    set({ sort, page: 0 });
  },
  setTrust: (trust) => {
    set({ trust, page: 0 });
  },
  setPage: (page) => {
    set({ page });
  },
  nextPage: async () => {
    const { page, hasMore } = get();
    if (hasMore) {
      await get().search(page + 1);
    }
  },
  prevPage: async () => {
    const { page } = get();
    if (page > 0) {
      await get().search(page - 1);
    }
  },
  search: async (targetPage?: number) => {
    const { query, sort, trust, page: currentPage } = get();
    const page = targetPage ?? currentPage;
    searchSequence += 1;
    const requestSequence = searchSequence;
    set({ searching: true, page });
    try {
      const response = await modsApi.search({ query, sort, trust, page });
      if (requestSequence !== searchSequence) {
        return;
      }
      set({ results: response.results, hasMore: response.hasMore, page: response.page });
      if (response.offline) {
        useUiStore.getState().notify({
          tone: "warning",
          title: "Tryb offline",
          message: "Wyświetlane są wyłącznie zapisane wyniki. Instalacja jest wyłączona.",
        });
      }
    } catch (error) {
      if (requestSequence === searchSequence) {
        notifyError(error);
      }
    } finally {
      if (requestSequence === searchSequence) {
        set({ searching: false });
      }
    }
  },
  refreshLocalState: async () => {
    set({ refreshing: true });
    try {
      const [installed, pending] = await Promise.all([
        modsApi.installed(),
        modsApi.pending(),
      ]);
      set({ installed, pending });
    } catch (error) {
      notifyError(error);
    } finally {
      set({ refreshing: false });
    }
  },
  prepareInstall: async (mod) => {
    // Guard against a slower earlier plan landing under a newer selection -
    // confirmInstall would otherwise install this mod against the dependency
    // list and disk usage the user reviewed for a different one.
    planSequence += 1;
    const requestSequence = planSequence;
    set({ selectedMod: mod, installPlan: null, mutatingProjectId: mod.projectId });
    useUiStore.getState().openModal("install-plan");
    try {
      const installPlan = await modsApi.installPlan(mod.projectId);
      if (requestSequence !== planSequence) {
        return;
      }
      set({ installPlan });
    } catch (error) {
      if (requestSequence !== planSequence) {
        return;
      }
      notifyError(error);
      set({ selectedMod: null });
      useUiStore.getState().closeModal();
    } finally {
      if (requestSequence === planSequence) {
        set({ mutatingProjectId: null });
      }
    }
  },
  closeInstallPlan: () => {
    set({ selectedMod: null, installPlan: null, operationProgress: null });
    useUiStore.getState().closeModal();
  },
  confirmInstall: async () => {
    const selected = get().selectedMod;
    if (!selected) {
      return;
    }
    set({ mutatingProjectId: selected.projectId });
    try {
      const result = await modsApi.install(selected.projectId, selected.versionId);
      useUiStore.getState().notify({
        tone: result.queued ? "warning" : "success",
        title: result.queued ? "Dodano do kolejki" : "Mod zainstalowany",
        message: result.message,
      });
      await Promise.all([get().refreshLocalState(), get().search()]);
      if (!result.queued) {
        get().closeInstallPlan();
      }
    } catch (error) {
      notifyError(error);
    } finally {
      set({ mutatingProjectId: null });
    }
  },
  remove: async (mod) => {
    set({ mutatingProjectId: mod.projectId });
    try {
      const result = await modsApi.remove(mod.projectId);
      useUiStore.getState().notify({
        tone: result.queued ? "warning" : "success",
        title: result.queued ? "Dodano do kolejki" : "Mod usunięty",
        message: result.message,
      });
      await Promise.all([get().refreshLocalState(), get().search()]);
    } catch (error) {
      notifyError(error);
    } finally {
      set({ mutatingProjectId: null });
    }
  },
  update: async (mod) => {
    set({ mutatingProjectId: mod.projectId });
    try {
      const result = await modsApi.update(mod.projectId);
      useUiStore.getState().notify({
        tone: result.queued ? "warning" : "success",
        title: result.queued ? "Dodano do kolejki" : "Mod zaktualizowany",
        message: result.message,
      });
      await Promise.all([get().refreshLocalState(), get().search()]);
    } catch (error) {
      notifyError(error);
    } finally {
      set({ mutatingProjectId: null });
    }
  },
  downloadOptifine: async () => {
    set({ mutatingProjectId: "local-private-pack" });
    try {
      const result = await modsApi.downloadOptifine();
      useUiStore.getState().notify({
        tone: "success",
        title: "Private Pack zainstalowany",
        message: result.message,
      });
      await get().refreshLocalState();
      useUiStore.getState().closeModal();
    } catch (error) {
      notifyError(error);
    } finally {
      set({ mutatingProjectId: null });
    }
  },
  importOptifine: async () => {
    set({ mutatingProjectId: "local-optifine" });
    try {
      const result = await modsApi.importOptifine();
      useUiStore.getState().notify({
        tone: result.queued ? "warning" : "success",
        title: result.queued ? "Dodano do kolejki" : "Import zakończony",
        message: result.message,
      });
      await get().refreshLocalState();
      useUiStore.getState().closeModal();
    } catch (error) {
      notifyError(error);
    } finally {
      set({ mutatingProjectId: null });
    }
  },
  cancelPending: async (operationId) => {
    try {
      await modsApi.cancelPending(operationId);
      await get().refreshLocalState();
    } catch (error) {
      notifyError(error);
    }
  },
  applyPending: async () => {
    try {
      const result = await modsApi.applyPending();
      useUiStore.getState().notify({
        tone: "success",
        title: "Kolejka zastosowana",
        message: result.message,
      });
      await get().refreshLocalState();
    } catch (error) {
      notifyError(error);
    }
  },
  applyOperationProgress: (operationProgress) => {
    set({ operationProgress });
  },
}));

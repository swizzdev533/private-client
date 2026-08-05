import { create } from "zustand";

export type MainTab = "play" | "mods";
export type ModsView = "library" | "installed";
export type ModalName = "settings" | "install-plan" | "optifine" | null;

export interface Notice {
  id: string;
  tone: "neutral" | "success" | "warning" | "error";
  title: string;
  message: string;
}

interface UiState {
  activeTab: MainTab;
  modsView: ModsView;
  modal: ModalName;
  notices: Notice[];
  setActiveTab: (tab: MainTab) => void;
  setModsView: (view: ModsView) => void;
  openModal: (modal: Exclude<ModalName, null>) => void;
  closeModal: () => void;
  notify: (notice: Omit<Notice, "id">) => void;
  dismissNotice: (id: string) => void;
}

let noticeSequence = 0;

export const useUiStore = create<UiState>((set) => ({
  activeTab: "play",
  modsView: "library",
  modal: null,
  notices: [],
  setActiveTab: (activeTab) => {
    set({ activeTab });
  },
  setModsView: (modsView) => {
    set({ modsView });
  },
  openModal: (modal) => {
    set({ modal });
  },
  closeModal: () => {
    set({ modal: null });
  },
  notify: (notice) => {
    noticeSequence += 1;
    const id = `notice-${noticeSequence}`;
    set((state) => ({
      notices: [...state.notices.slice(-3), { ...notice, id }],
    }));
    window.setTimeout(() => {
      set((state) => ({
        notices: state.notices.filter((item) => item.id !== id),
      }));
    }, 5_500);
  },
  dismissNotice: (id) => {
    set((state) => ({
      notices: state.notices.filter((notice) => notice.id !== id),
    }));
  },
}));

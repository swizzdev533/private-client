import type { ReactNode } from "react";
import { motion } from "framer-motion";
import {
  Gamepad2,
  LibraryBig,
  Minus,
  Settings,
  Square,
  X,
} from "lucide-react";
import { isTauriRuntime } from "../../lib/tauriBridge";
import { windowApi } from "../../services/windowApi";
import { useUiStore, type MainTab } from "../../stores/useUiStore";

interface AppShellProps {
  children: ReactNode;
  appVersion: string;
}

const tabs: Array<{
  id: MainTab;
  label: string;
  icon: typeof Gamepad2;
}> = [
  { id: "play", label: "PLAY", icon: Gamepad2 },
  { id: "mods", label: "MODS", icon: LibraryBig },
];

export function AppShell({ children, appVersion }: AppShellProps) {
  const activeTab = useUiStore((state) => state.activeTab);
  const setActiveTab = useUiStore((state) => state.setActiveTab);
  const openModal = useUiStore((state) => state.openModal);

  return (
    <div className="app-shell">
      <header className="topbar" data-tauri-drag-region>
        <div className="brand" aria-label="Private Client" data-tauri-drag-region>
          <div className="brand__copy" data-tauri-drag-region>
            <strong>PRIVATE CLIENT</strong>
          </div>
        </div>

        <nav className="main-nav" aria-label="Główna nawigacja">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const selected = activeTab === tab.id;
            return (
              <button
                type="button"
                key={tab.id}
                className={`main-nav__item ${selected ? "is-active" : ""}`}
                onClick={() => {
                  setActiveTab(tab.id);
                }}
                aria-current={selected ? "page" : undefined}
              >
                {selected ? (
                  <motion.span
                    layoutId="main-nav-active"
                    className="main-nav__active"
                    transition={{ type: "spring", stiffness: 360, damping: 32 }}
                  />
                ) : null}
                <Icon size={16} strokeWidth={1.8} aria-hidden="true" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="topbar__actions">
          <div className="topbar__utilities">
            <button
              type="button"
              className="topbar__utility"
              aria-label="Otwórz ustawienia"
              title="Ustawienia"
              onClick={() => {
                openModal("settings");
              }}
            >
              <Settings size={15} strokeWidth={1.8} aria-hidden="true" />
            </button>
          </div>

          <div className="window-controls" aria-label="Sterowanie oknem">
            <button
              type="button"
              disabled={!isTauriRuntime}
              aria-label="Minimalizuj okno"
              title="Minimalizuj"
              onClick={() => {
                void windowApi.minimize();
              }}
            >
              <Minus size={15} aria-hidden="true" />
            </button>
            <button
              type="button"
              disabled={!isTauriRuntime}
              aria-label="Maksymalizuj lub przywróć okno"
              title="Maksymalizuj / przywróć"
              onClick={() => {
                void windowApi.toggleMaximize();
              }}
            >
              <Square size={12} aria-hidden="true" />
            </button>
            <button
              type="button"
              className="window-control--close"
              disabled={!isTauriRuntime}
              aria-label="Zamknij aplikację"
              title="Zamknij"
              onClick={() => {
                void windowApi.close();
              }}
            >
              <X size={15} aria-hidden="true" />
            </button>
          </div>
        </div>
      </header>

      <main className="app-main">{children}</main>

      <footer className="app-footer">
        <span>Private Client is not affiliated with Mojang Studios or Microsoft.</span>
        <span>v{appVersion}</span>
      </footer>
    </div>
  );
}

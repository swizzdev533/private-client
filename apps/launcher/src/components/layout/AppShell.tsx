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
  channel: "stable" | "beta";
}

const tabs: Array<{
  id: MainTab;
  label: string;
  icon: typeof Gamepad2;
}> = [
  { id: "play", label: "PLAY", icon: Gamepad2 },
  { id: "mods", label: "MODS", icon: LibraryBig },
];

export function AppShell({ children, appVersion, channel }: AppShellProps) {
  const activeTab = useUiStore((state) => state.activeTab);
  const setActiveTab = useUiStore((state) => state.setActiveTab);
  const openModal = useUiStore((state) => state.openModal);

  return (
    <div className="app-shell">
      <header className="topbar" data-tauri-drag-region>
        <div className="brand" aria-label="Private Client" data-tauri-drag-region>
          <div className="brand__copy" data-tauri-drag-region>
            <strong>PRIVATE CLIENT</strong>
            {/* Product line, not the build version: the footer carries the real
                version, and this must not be read as one. */}
            <span>BETA 1.0</span>
          </div>
        </div>

        <nav className="main-nav" aria-label="Main navigation">
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
              aria-label="Open settings"
              title="Settings"
              onClick={() => {
                openModal("settings");
              }}
            >
              <Settings size={15} strokeWidth={1.8} aria-hidden="true" />
            </button>
          </div>

          <div className="window-controls" aria-label="Window controls">
            <button
              type="button"
              disabled={!isTauriRuntime}
              aria-label="Minimize window"
              title="Minimize"
              onClick={() => {
                void windowApi.minimize();
              }}
            >
              <Minus size={15} aria-hidden="true" />
            </button>
            <button
              type="button"
              disabled={!isTauriRuntime}
              aria-label="Maximize or restore window"
              title="Maximize / restore"
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
              aria-label="Close application"
              title="Close"
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
        <span>
          v{appVersion}
          {/* The window has no OS title bar, so this is the only in-app way to
              tell the beta install apart from the stable one. */}
          {channel === "beta" ? <strong className="app-footer__channel">TEST BUILD</strong> : null}
        </span>
      </footer>
    </div>
  );
}

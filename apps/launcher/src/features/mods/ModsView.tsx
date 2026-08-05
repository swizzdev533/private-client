import { useEffect } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Boxes,
  ChevronLeft,
  ChevronRight,
  FolderInput,
  LibraryBig,
  PackageCheck,
  RefreshCcw,
  Search,
} from "lucide-react";
import { Button } from "../../components/common/Button";
import { EmptyState } from "../../components/common/EmptyState";
import { useModsStore } from "../../stores/useModsStore";
import { useUiStore } from "../../stores/useUiStore";
import type { InstalledMod, ModSummary } from "../../types/contracts";
import { ModCard } from "./ModCard";
import { InstalledModCard } from "./InstalledModCard";
import { PendingQueue } from "./PendingQueue";

interface ModsViewProps {
  reducedMotion: boolean;
}

function CardSkeletons() {
  return (
    <div className="mods-grid" aria-label="Loading mods">
      {[0, 1, 2, 3, 4, 5].map((item) => (
        <div className="mod-card mod-card--skeleton" key={item}>
          <div className="skeleton" />
          <div className="skeleton" />
          <div className="skeleton" />
        </div>
      ))}
    </div>
  );
}

export function ModsView({ reducedMotion }: ModsViewProps) {
  const view = useUiStore((state) => state.modsView);
  const setView = useUiStore((state) => state.setModsView);
  const openModal = useUiStore((state) => state.openModal);
  const query = useModsStore((state) => state.query);
  const page = useModsStore((state) => state.page);
  const hasMore = useModsStore((state) => state.hasMore);
  const results = useModsStore((state) => state.results);
  const installed = useModsStore((state) => state.installed);
  const pending = useModsStore((state) => state.pending);
  const searching = useModsStore((state) => state.searching);
  const refreshing = useModsStore((state) => state.refreshing);
  const mutatingProjectId = useModsStore((state) => state.mutatingProjectId);
  const setQuery = useModsStore((state) => state.setQuery);
  const nextPage = useModsStore((state) => state.nextPage);
  const prevPage = useModsStore((state) => state.prevPage);
  const search = useModsStore((state) => state.search);
  const refreshLocalState = useModsStore((state) => state.refreshLocalState);
  const prepareInstall = useModsStore((state) => state.prepareInstall);
  const remove = useModsStore((state) => state.remove);
  const update = useModsStore((state) => state.update);
  const cancelPending = useModsStore((state) => state.cancelPending);
  const applyPending = useModsStore((state) => state.applyPending);

  useEffect(() => {
    void Promise.all([search(), refreshLocalState()]);
  }, [refreshLocalState, search]);

  const updateFromLibrary = (mod: ModSummary) => {
    const local = installed.find((item) => item.projectId === mod.projectId);
    if (local) {
      void update(local);
    }
  };

  return (
    <motion.div
      className="page"
      initial={reducedMotion ? false : { opacity: 0, x: 16 }}
      animate={{ opacity: 1, x: 0 }}
      exit={reducedMotion ? {} : { opacity: 0, x: -12 }}
      transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
    >
      <div className="page__inner mods-page">
        <header className="mods-header">
          <div className="mods-header__copy">
            <h1>Mod library</h1>
            <p>
              Search official projects, check compatibility, and install them together
              with their dependencies in one safe transaction.
            </p>
          </div>
          <div className="mods-header__actions">
            <Button
              variant="ghost"
              className="button--optifine-glow"
              icon={<FolderInput size={16} />}
              onClick={() => {
                openModal("optifine");
              }}
            >
              IMPORT PRIVATE PACK
            </Button>
            <Button
              variant="secondary"
              icon={<RefreshCcw size={16} />}
              busy={refreshing}
              onClick={() => {
                void Promise.all([search(), refreshLocalState()]);
              }}
            >
              REFRESH
            </Button>
          </div>
        </header>

        <div className="mods-toolbar">
          <div className="mods-subviews" role="tablist" aria-label="Mod view">
            <button
              type="button"
              role="tab"
              aria-selected={view === "library"}
              className={view === "library" ? "is-active" : ""}
              onClick={() => {
                setView("library");
              }}
            >
              <LibraryBig size={15} />
              LIBRARY
              <span>{results.length}</span>
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={view === "installed"}
              className={view === "installed" ? "is-active" : ""}
              onClick={() => {
                setView("installed");
              }}
            >
              <PackageCheck size={15} />
              INSTALLED MODS
              <span>{installed.length}</span>
            </button>
          </div>

          {view === "library" ? (
            <form
              className="mod-search"
              onSubmit={(event) => {
                event.preventDefault();
                void search();
              }}
            >
              <Search size={17} aria-hidden="true" />
              <input
                type="search"
                placeholder="Search Modrinth for a mod…"
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                }}
                aria-label="Search mods"
              />
              <Button type="submit" variant="primary" size="sm" busy={searching}>
                SEARCH
              </Button>
            </form>
          ) : null}
        </div>

        <PendingQueue
          operations={pending}
          onCancel={(id) => {
            void cancelPending(id);
          }}
          onApply={() => {
            void applyPending();
          }}
        />

        <AnimatePresence mode="wait">
          {view === "library" ? (
            <motion.section
              key="library"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              aria-label="Library"
            >
              {searching && results.length === 0 ? (
                <CardSkeletons />
              ) : results.length > 0 ? (
                <>
                  <div className="mods-grid">
                    {results.map((mod) => (
                      <ModCard
                        // versionId is the literal "unavailable" for every hit
                        // without a downloadable version, so it is not unique.
                        key={mod.projectId}
                        mod={mod}
                        busy={mutatingProjectId === mod.projectId}
                        onInstall={(selected) => {
                          void prepareInstall(selected);
                        }}
                        onUpdateInstalled={updateFromLibrary}
                      />
                    ))}
                  </div>
                  <div className="mods-pagination">
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={page === 0 || searching}
                      icon={<ChevronLeft size={16} />}
                      onClick={() => {
                        void prevPage();
                      }}
                    >
                      PREVIOUS PAGE
                    </Button>
                    <span className="mods-pagination__info">Page {page + 1}</span>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={!hasMore || searching}
                      icon={<ChevronRight size={16} />}
                      onClick={() => {
                        void nextPage();
                      }}
                    >
                      NEXT PAGE
                    </Button>
                  </div>
                </>
              ) : (
                <EmptyState
                  icon={<Search size={22} />}
                  title="No compatible results"
                  description="Change the search term or filters. Private Client will not offer a file as installable unless it matches Forge 1.8.9."
                  action={
                    <Button
                      variant="secondary"
                      onClick={() => {
                        setQuery("");
                        void search();
                      }}
                    >
                      CLEAR SEARCH
                    </Button>
                  }
                />
              )}
            </motion.section>
          ) : (
            <motion.section
              key="installed"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              aria-label="Installed Mods"
            >
              {refreshing && installed.length === 0 ? (
                <CardSkeletons />
              ) : installed.length > 0 ? (
                <div className="mods-grid">
                  {installed.map((mod) => (
                    <InstalledModCard
                      key={`${mod.projectId}-${mod.fileName}`}
                      mod={mod}
                      busy={mutatingProjectId === mod.projectId}
                      onRemove={(selected: InstalledMod) => {
                        void remove(selected);
                      }}
                      onUpdate={(selected: InstalledMod) => {
                        void update(selected);
                      }}
                    />
                  ))}
                </div>
              ) : (
                <EmptyState
                  icon={<Boxes size={22} />}
                  title="No optional mods"
                  description="Go back to Library and pick a compatible project. Required client components appear automatically."
                  action={
                    <Button
                      variant="secondary"
                      onClick={() => {
                        setView("library");
                      }}
                    >
                      OPEN LIBRARY
                    </Button>
                  }
                />
              )}
            </motion.section>
          )}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { resetDemoBackend } from "./lib/demoBackend";
import { useLauncherStore } from "./stores/useLauncherStore";
import { useModsStore } from "./stores/useModsStore";
import { useUiStore } from "./stores/useUiStore";

function resetStores(): void {
  useUiStore.setState({
    activeTab: "play",
    modsView: "library",
    modal: null,
    notices: [],
  });
  useLauncherStore.setState({
    snapshot: null,
    initializing: true,
    actionPending: false,
    update: null,
    updateChecking: false,
    updateInstalling: false,
  });
  useModsStore.setState({
    query: "",
    sort: "relevance",
    trust: "all",
    results: [],
    installed: [],
    pending: [],
    selectedMod: null,
    installPlan: null,
    operationProgress: null,
    searching: false,
    refreshing: false,
    mutatingProjectId: null,
  });
}

describe("Private Client launcher", () => {
  beforeEach(() => {
    resetDemoBackend();
    resetStores();
  });

  afterEach(() => {
    cleanup();
  });

  it("renders exactly two main tabs and the profile fallback", async () => {
    render(<App />);

    const navigation = screen.getByRole("navigation", {
      name: "Main navigation",
    });
    const mainTabs = within(navigation).getAllByRole("button");
    expect(mainTabs).toHaveLength(2);
    expect(mainTabs[0]).toHaveTextContent("PLAY");
    expect(mainTabs[1]).toHaveTextContent("MODS");

    expect(await screen.findByTestId("launch-action")).toBeVisible();
    expect(screen.queryByRole("tab", { name: /settings/i })).not.toBeInTheDocument();
    expect(screen.queryByText("BROWSER DEMO · PREVIEW")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Minimize window" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Maximize or restore window" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Close application" })).toBeDisabled();
  });

  it("switches to Library and searches the browser preview catalogue", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "MODS" }));
    expect(await screen.findByRole("tab", { name: /LIBRARY/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    const input = screen.getByRole("searchbox", { name: "Search mods" });
    await user.type(input, "FoamFix");
    await user.click(screen.getByRole("button", { name: "SEARCH" }));

    expect(await screen.findByText("FoamFix Legacy")).toBeVisible();
    await waitFor(() => {
      expect(screen.queryByText("Patcher")).not.toBeInTheDocument();
    });
  });

  it("protects required installed mods from removal", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "MODS" }));
    const installedTab = await screen.findByRole("tab", {
      name: /INSTALLED MODS/,
    });
    await user.click(installedTab);

    const core = await screen.findByText("Private Optimization");
    const card = core.closest("article");
    expect(card).not.toBeNull();
    expect(within(card!).getByRole("button", { name: "REMOVE" })).toBeDisabled();
    await waitFor(() => {
      expect(within(card!).getByText(/required by Private Client/i)).toBeVisible();
    });
  });

  it("clears the local profile when the profile event carries null", async () => {
    render(<App />);
    expect(await screen.findByTestId("launch-action")).toBeVisible();

    act(() => {
      useLauncherStore.getState().applyProfile({
        schemaVersion: 1,
        username: "LogoutProof",
        uuid: "00000000-0000-0000-0000-000000000001",
        skinModel: "slim",
        skinPath: null,
        updatedAt: "2026-07-30T16:00:00.000Z",
      });
    });

    act(() => {
      useLauncherStore.getState().applyProfile(null);
    });
    expect(await screen.findByTestId("launch-action")).toBeVisible();
    expect(screen.queryByText("LogoutProof")).not.toBeInTheDocument();
  });
});

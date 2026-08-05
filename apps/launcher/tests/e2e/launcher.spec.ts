import { mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test, type Locator, type Page, type TestInfo } from "@playwright/test";

const evidenceDirectory = resolve(process.cwd(), "../../artifacts/evidence");

function mainNavigation(page: Page): Locator {
  return page.getByRole("navigation", { name: "Główna nawigacja" });
}

async function openMods(page: Page): Promise<void> {
  await mainNavigation(page).getByRole("button", { name: "MODS" }).click();
  await expect(page.getByRole("tab", { name: /LIBRARY/ })).toBeVisible();
}

async function searchLibrary(page: Page, query: string): Promise<void> {
  const input = page.getByRole("searchbox", { name: "Wyszukaj mod" });
  await input.fill(query);
  await page.getByRole("button", { name: "SZUKAJ" }).click();
}

function modCard(page: Page, name: string): Locator {
  return page.locator("article.mod-card").filter({ hasText: name });
}

async function attachScreenshot(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Promise<void> {
  await mkdir(evidenceDirectory, { recursive: true });
  const screenshot = await page.screenshot({
    path: join(evidenceDirectory, `${name}.png`),
    fullPage: true,
  });
  await testInfo.attach(name, {
    body: screenshot,
    contentType: "image/png",
  });
}

test("01–03 launcher start, no-profile state and MODS navigation", async ({
  page,
}, testInfo) => {
  await test.step("01 start launchera", async () => {
    await page.goto("/");
    await expect(page.getByLabel("Private Client")).toBeVisible();
    await expect(mainNavigation(page).getByRole("button")).toHaveCount(2);
    await expect(page.getByText("BROWSER DEMO · PODGLĄD")).toBeVisible();
  });

  await test.step("02 ekran bez profilu", async () => {
    await expect(page.getByText("Zaloguj się w grze")).toBeVisible();
    await expect(page.getByTestId("launch-action")).toBeEnabled();
    await attachScreenshot(page, testInfo, "play-browser-demo");
  });

  await test.step("03 przejście do MODS", async () => {
    await openMods(page);
    await expect(page.getByRole("tab", { name: /LIBRARY/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await attachScreenshot(page, testInfo, "mods-browser-demo");
  });
});

test("04–10 fixture search, install, local management and download failure", async ({
  page,
}) => {
  await page.goto("/");
  await openMods(page);

  await test.step("04 wyszukanie fixture moda", async () => {
    await searchLibrary(page, "FoamFix");
    await expect(modCard(page, "FoamFix Legacy")).toBeVisible();
    await expect(modCard(page, "FoamFix Legacy")).toContainText("COMPATIBLE");
  });

  await test.step("05 instalacja z zatwierdzeniem planu", async () => {
    await modCard(page, "FoamFix Legacy").getByRole("button", { name: "INSTALL" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toContainText("Zainstaluj FoamFix Legacy");
    await expect(dialog).toContainText("TRANSAKCJA ATOMOWA");
    await dialog.getByRole("button", { name: "ZATWIERDŹ I ZAINSTALUJ" }).click();
    await expect(page.getByText("Mod zainstalowany")).toBeVisible({
      timeout: 6_000,
    });
    await expect(dialog).toBeHidden();
  });

  await test.step("06 mod pojawia się w Installed Mods", async () => {
    await page.getByRole("tab", { name: /INSTALLED MODS/ }).click();
    const installedFoamFix = page
      .locator("article.installed-card")
      .filter({ hasText: "FoamFix Legacy" });
    await expect(installedFoamFix).toBeVisible();
    await expect(installedFoamFix).toContainText("0.6.3");
  });

  await test.step("07 próba usunięcia wymaganego moda jest blokowana", async () => {
    const required = page
      .locator("article.installed-card")
      .filter({ hasText: "Private Client Core" });
    await expect(required).toBeVisible();
    await expect(required).toContainText("REQUIRED");
    await expect(required.getByRole("button", { name: "REMOVE" })).toBeDisabled();
    await expect(required).toContainText("wymagany przez Private Client");
  });

  await test.step("08 aktualizacja zainstalowanego moda", async () => {
    const patcher = page.locator("article.installed-card").filter({ hasText: "Patcher" });
    await expect(patcher.getByRole("button", { name: "UPDATE" })).toBeVisible();
    await patcher.getByRole("button", { name: "UPDATE" }).click();
    await expect(page.getByText("Mod zaktualizowany")).toBeVisible({
      timeout: 6_000,
    });
    await expect(patcher).toContainText("1.8.7");
    await expect(patcher.getByRole("button", { name: "UPDATE" })).toHaveCount(0);
  });

  await test.step("09 kontrolowany błąd pobierania", async () => {
    await page.getByRole("tab", { name: /^LIBRARY/ }).click();
    await searchLibrary(page, "Fixture Download Failure");
    const fixture = modCard(page, "Fixture Download Failure");
    await expect(fixture).toBeVisible();
    await fixture.getByRole("button", { name: "INSTALL" }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "ZATWIERDŹ I ZAINSTALUJ" }).click();
    await expect(page.getByText(/DownloadFailed/)).toBeVisible({
      timeout: 5_000,
    });
    await expect(
      page.getByText("Kontrolowany błąd pobierania z fixture E2E."),
    ).toBeVisible();
    await dialog.getByRole("button", { name: "ANULUJ" }).click();
  });

  await test.step("10 powrót do PLAY", async () => {
    await mainNavigation(page).getByRole("button", { name: "PLAY" }).click();
    await expect(page.getByTestId("launch-action")).toBeVisible();
    // Panel stanu instancji pojawia się tylko przy błędzie uruchamiania.
    await expect(page.getByText("Stan instancji")).toHaveCount(0);
  });
});

test("11–13 launch state machine, crash summary and local logs", async ({
  page,
}, testInfo) => {
  await page.goto("/?e2eCrash=1");
  const launchState = page.getByTestId("launch-state");

  await test.step("11 testowa maszyna stanów uruchamiania", async () => {
    await expect(launchState).toHaveAttribute("data-state", "IDLE");
    await page.getByTestId("launch-action").click();
    await expect(launchState).toHaveAttribute("data-state", "VALIDATING", {
      timeout: 1_500,
    });
    await expect(launchState).toHaveAttribute("data-state", "RUNNING", {
      timeout: 5_000,
    });
    // Panel stanu jest ukryty w trakcie uruchamiania — stan potwierdza przycisk.
    await expect(page.getByTestId("launch-action")).toContainText("OTWÓRZ GRĘ");
  });

  await test.step("12 obsługa kontrolowanego crashu Forge", async () => {
    await expect(launchState).toHaveAttribute("data-state", "FAILED", {
      timeout: 3_000,
    });
    await expect(page.getByText("Kontrolowany crash Forge z fixture E2E")).toBeVisible();
    await expect(page.getByText("Uruchamianie nie powiodło się")).toBeVisible();
    await attachScreenshot(page, testInfo, "crash-browser-demo");
  });
});

import { test, expect, type Page } from "@playwright/test";

// `ThemeToggle` reads `localStorage` through `useSyncExternalStore` rather than
// mirroring it into `useState`. These tests pin the behaviour that change was
// made for: the first client render agrees with the server (no correcting
// re-render), a stored choice outranks the system preference, and a write from
// another tab moves this one.

// The live toggle is labelled "Switch to <x> mode"; the pre-hydration
// placeholder is labelled "Toggle theme". Waiting for this selector therefore
// also proves hydration finished.
const live = (page: Page) => page.locator('button[aria-label^="Switch to"]');

const themeOf = (page: Page) =>
  page.locator("html").getAttribute("data-theme");

test.describe("theme toggle", () => {
  for (const scheme of ["dark", "light"] as const) {
    test(`with no stored choice, follows prefers-color-scheme: ${scheme}`, async ({
      page,
    }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto("/");
      await live(page).waitFor();
      expect(await themeOf(page)).toBe(scheme);
    });
  }

  test("a stored choice outranks the system preference", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.addInitScript(() => localStorage.setItem("theme", "light"));
    await page.goto("/");
    await live(page).waitFor();
    expect(await themeOf(page)).toBe("light");
  });

  test("clicking flips the theme, persists it, and flips back", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/");
    const button = live(page);
    await button.waitFor();
    await expect(button).toHaveAttribute("aria-label", "Switch to light mode");

    await button.click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await expect(button).toHaveAttribute("aria-label", "Switch to dark mode");
    expect(await page.evaluate(() => localStorage.getItem("theme"))).toBe(
      "light",
    );

    await button.click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    expect(await page.evaluate(() => localStorage.getItem("theme"))).toBe(
      "dark",
    );
  });

  // The capability the old `useState` version did not have: a `storage` event
  // is what another tab's write looks like to this page.
  test("picks up a theme change made in another tab", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/");
    await live(page).waitFor();
    expect(await themeOf(page)).toBe("dark");

    await page.evaluate(() => {
      localStorage.setItem("theme", "light");
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "theme",
          newValue: "light",
          storageArea: localStorage,
        }),
      );
    });

    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  });

  test("the choice survives a reload", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/");
    await live(page).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

    await page.reload();
    await live(page).waitFor();
    expect(await themeOf(page)).toBe("light");
  });

  test("hydrates without a mismatch", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
    page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));

    await page.goto("/");
    await live(page).waitFor();

    const hydration = errors.filter((e) =>
      /hydrat|did not match|Text content/i.test(e),
    );
    expect(hydration, hydration.join(" | ")).toHaveLength(0);
  });
});

import { test, expect } from "@playwright/test";

test("home locale switcher changes locale while preserving path", async ({ page }) => {
  await page.goto("/en-US/");
  await expect(page.locator("html")).toHaveAttribute("lang", "en-US");

  await page.selectOption("#locale-switcher", "fr-FR");
  await expect(page).toHaveURL(/\/fr-FR\/?$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-FR");
});

test("docs: locale switcher preserves slug and shows fallback notice when untranslated", async ({ page }) => {
  await page.goto("/en-US/docs/mission");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(/Mission/);

  await page.selectOption("#locale-switcher", "ja-JP");
  await expect(page).toHaveURL(/\/ja-JP\/docs\/mission\/?$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "ja-JP");
  await expect(page.getByRole("status")).toBeVisible();
});

test("blog: navigate to an article and keep slug when switching locales", async ({ page }) => {
  await page.goto("/en-US/blog");

  const firstPostLink = page.locator("main ul li a").first();
  const href = await firstPostLink.getAttribute("href");
  expect(href).toMatch(/^\/en-US\/blog\/.+/);
  await firstPostLink.click();

  await expect(page).toHaveURL(/\/en-US\/blog\/[^/]+\/?$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();

  await page.selectOption("#locale-switcher", "zh-CN");
  await expect(page).toHaveURL(/\/zh-CN\/blog\/[^/]+\/?$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.getByRole("status")).toBeVisible();
});

test("SEO: pages include canonical + hreflang alternates", async ({ page }) => {
  await page.goto("/en-US/docs/mission");
  const canonical = page.locator("head link[rel='canonical']");
  await expect(canonical).toHaveAttribute("href", /\/en-US\/docs\/mission$/);

  const alternates = page.locator("head link[rel='alternate'][hreflang]");
  await expect(alternates).toHaveCount(10);
});


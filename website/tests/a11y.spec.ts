import { test, expect } from "@playwright/test";
import type { Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function expectNoA11yViolations(page: Page) {
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
}

test("home has no obvious a11y violations", async ({ page }) => {
  await page.goto("/en-US/");
  await expect(page.getByRole("link", { name: "Skip to content" })).toHaveAttribute("href", "#main");
  await expect(page.getByRole("navigation", { name: "Primary" })).toBeVisible();
  await expectNoA11yViolations(page);
});

test("root redirects to default locale", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/en-US\/?$/);
});

test("blog index has no obvious a11y violations", async ({ page }) => {
  await page.goto("/en-US/blog");
  await expect(page.getByRole("heading", { level: 1, name: "Blog" })).toBeVisible();
  await expectNoA11yViolations(page);
});

test("docs mission has no obvious a11y violations", async ({ page }) => {
  await page.goto("/en-US/docs/mission");
  await expect(page.getByRole("heading", { level: 1, name: "Mission" })).toBeVisible();
  await expectNoA11yViolations(page);
});

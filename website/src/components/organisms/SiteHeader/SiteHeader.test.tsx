import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

let pathnameNoLocale = "/docs/mission";
const hasDocTranslation = vi.fn((locale: string, _slug: string[]) => locale === "en-US");
const hasBlogPostTranslation = vi.fn((locale: string, _slug: string) => locale === "en-US");

vi.mock("next-intl/server", () => ({
  getTranslations: async ({ namespace }: { namespace: string }) => {
    return (key: string) => {
      if (namespace === "localeNames") return key;
      if (namespace === "i18n" && key === "fallbackOptionSuffix") return " (fallback)";
      return key;
    };
  }
}));

vi.mock("next/headers", () => ({
  headers: async () => new Map([["x-chic-pathname-no-locale", pathnameNoLocale]])
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn() }),
  usePathname: () => "/en-US/docs/mission",
  useSearchParams: () => new URLSearchParams()
}));

vi.mock("@/lib/contentAvailability", () => ({
  hasDocTranslation: (locale: string, slug: string[]) => hasDocTranslation(locale, slug),
  hasBlogPostTranslation: (locale: string, slug: string) => hasBlogPostTranslation(locale, slug)
}));

import { SiteHeader } from "@/components/organisms/SiteHeader/SiteHeader";

describe("<SiteHeader />", () => {
  beforeEach(() => {
    hasDocTranslation.mockClear();
    hasBlogPostTranslation.mockClear();
  });

  it("renders navigation and marks missing translations in the locale picker", async () => {
    pathnameNoLocale = "/docs/mission";
    hasDocTranslation.mockImplementation((locale: string) => locale === "en-US");
    hasBlogPostTranslation.mockImplementation(() => true);

    const element = await SiteHeader({ locale: "en-US" });
    render(element);

    expect(screen.getByRole("link", { name: "skipToContent" })).toHaveAttribute("href", "#main");
    expect(screen.getByRole("navigation", { name: "primaryNav" })).toBeInTheDocument();

    // One of the non-default locales should have a fallback suffix when translation is missing.
    expect(screen.getByRole("option", { name: "fr-FR (fallback)" })).toBeInTheDocument();
  });

  it("uses blog slug translation checks when on a blog article", async () => {
    pathnameNoLocale = "/blog/hello-from-chic";
    hasDocTranslation.mockImplementation(() => true);
    hasBlogPostTranslation.mockImplementation((locale: string) => locale === "en-US");

    const element = await SiteHeader({ locale: "en-US" });
    render(element);

    expect(hasBlogPostTranslation).toHaveBeenCalled();
    expect(screen.getByRole("option", { name: "es-ES (fallback)" })).toBeInTheDocument();
  });

  it("does not show fallback markers when there is no per-page translation check", async () => {
    pathnameNoLocale = "/";
    hasDocTranslation.mockImplementation(() => false);
    hasBlogPostTranslation.mockImplementation(() => false);

    const element = await SiteHeader({ locale: "en-US" });
    render(element);

    expect(screen.getByRole("option", { name: "fr-FR" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "fr-FR (fallback)" })).toBeNull();
  });

  it("falls back to '/' when request header is missing and avoids per-page checks", async () => {
    pathnameNoLocale = undefined as any;
    hasDocTranslation.mockImplementation(() => false);
    hasBlogPostTranslation.mockImplementation(() => false);

    const element = await SiteHeader({ locale: "en-US" });
    render(element);

    expect(screen.getByRole("option", { name: "ja-JP" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "ja-JP (fallback)" })).toBeNull();
  });

  it("does not treat /blog/ as a blog article slug", async () => {
    pathnameNoLocale = "/blog/";
    hasDocTranslation.mockImplementation(() => false);
    hasBlogPostTranslation.mockImplementation(() => false);

    const element = await SiteHeader({ locale: "en-US" });
    render(element);

    expect(hasBlogPostTranslation).not.toHaveBeenCalled();
    expect(screen.getByRole("option", { name: "tr-TR" })).toBeInTheDocument();
  });
});

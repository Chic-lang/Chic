import { describe, expect, it } from "vitest";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";
import { DEFAULT_LOCALE } from "@/i18n/locales";

describe("withLocale", () => {
  it("prefixes paths with the locale", () => {
    expect(withLocale("en-US", "/docs")).toBe("/en-US/docs");
    expect(withLocale("ja-JP", "docs")).toBe("/ja-JP/docs");
  });

  it("treats / as the locale root", () => {
    expect(withLocale("en-US", "/")).toBe("/en-US");
  });
});

describe("stripLocaleFromPathname", () => {
  it("strips supported locale prefixes and normalizes trailing slashes", () => {
    expect(stripLocaleFromPathname("/en-US/docs/mission/")).toEqual({ locale: "en-US", pathname: "/docs/mission" });
    expect(stripLocaleFromPathname("fr-FR/blog/")).toEqual({ locale: "fr-FR", pathname: "/blog" });
    expect(stripLocaleFromPathname("/en-US/")).toEqual({ locale: "en-US", pathname: "/" });
  });

  it("uses default locale when no locale prefix exists", () => {
    expect(stripLocaleFromPathname("/docs")).toEqual({ locale: DEFAULT_LOCALE, pathname: "/docs" });
  });
});

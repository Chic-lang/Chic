import { describe, expect, it } from "vitest";
import { pickLocaleFromAcceptLanguage } from "@/i18n/acceptLanguage";
import { DEFAULT_LOCALE } from "@/i18n/locales";

describe("pickLocaleFromAcceptLanguage", () => {
  it("returns default locale when header is null", () => {
    expect(pickLocaleFromAcceptLanguage(null)).toBe(DEFAULT_LOCALE);
  });

  it("picks the first supported language in order, ignoring q-values", () => {
    expect(pickLocaleFromAcceptLanguage("fr-CA, fr;q=0.9, en;q=0.8")).toBe("fr-FR");
    expect(pickLocaleFromAcceptLanguage("de, es;q=0.9, en-US;q=0.8")).toBe("es-ES");
  });

  it("falls back to default locale when no supported language is present", () => {
    expect(pickLocaleFromAcceptLanguage("de, ko-KR;q=0.9")).toBe(DEFAULT_LOCALE);
  });

  it("skips malformed language tags", () => {
    expect(pickLocaleFromAcceptLanguage("-, es;q=0.9")).toBe("es-ES");
  });
});

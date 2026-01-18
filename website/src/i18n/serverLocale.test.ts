import { describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  notFound: () => {
    throw new Error("NOT_FOUND");
  }
}));

describe("getLocaleFromParams", () => {
  it("returns locale when valid", async () => {
    const { getLocaleFromParams } = await import("@/i18n/serverLocale");
    await expect(getLocaleFromParams(Promise.resolve({ locale: "ja-JP" }))).resolves.toBe("ja-JP");
  });

  it("calls notFound when invalid", async () => {
    const { getLocaleFromParams } = await import("@/i18n/serverLocale");
    await expect(getLocaleFromParams(Promise.resolve({ locale: "xx-XX" } as any))).rejects.toThrow("NOT_FOUND");
  });
});


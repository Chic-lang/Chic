import { describe, expect, it, vi } from "vitest";

vi.mock("next-intl/server", () => ({
  getRequestConfig: (fn: unknown) => fn
}));

describe("i18n request config", () => {
  it("returns requested locale when valid", async () => {
    const { default: getRequestConfig } = await import("@/i18n/request");
    const config = await (getRequestConfig as any)({ requestLocale: "es-ES" });
    expect(config.locale).toBe("es-ES");
    expect((config.messages as any).site?.name).toBeTypeOf("string");
  });

  it("falls back to default locale when invalid", async () => {
    const { default: getRequestConfig } = await import("@/i18n/request");
    const config = await (getRequestConfig as any)({ requestLocale: "xx-XX" });
    expect(config.locale).toBe("en-US");
  });
});


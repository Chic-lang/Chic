import { describe, expect, it } from "vitest";
import { loadMessages } from "@/i18n/messages";
import type { Locale } from "@/i18n/locales";

const LOCALES: Locale[] = ["en-US", "es-ES", "fr-FR", "it-IT", "ja-JP", "pt-BR", "ru-RU", "tr-TR", "zh-CN"];

describe("loadMessages", () => {
  it("loads message catalogs for every supported locale", async () => {
    for (const locale of LOCALES) {
      const messages = (await loadMessages(locale)) as any;
      expect(messages.site?.name).toBeTypeOf("string");
      expect(messages.a11y?.skipToContent).toBeTypeOf("string");
    }
  });
});


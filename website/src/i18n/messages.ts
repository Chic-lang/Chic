import type { AbstractIntlMessages } from "next-intl";
import type { Locale } from "@/i18n/locales";

export async function loadMessages(locale: Locale): Promise<AbstractIntlMessages> {
  switch (locale) {
    case "en-US":
      return (await import("@/messages/en-US.json")).default;
    case "es-ES":
      return (await import("@/messages/es-ES.json")).default;
    case "fr-FR":
      return (await import("@/messages/fr-FR.json")).default;
    case "it-IT":
      return (await import("@/messages/it-IT.json")).default;
    case "ja-JP":
      return (await import("@/messages/ja-JP.json")).default;
    case "pt-BR":
      return (await import("@/messages/pt-BR.json")).default;
    case "ru-RU":
      return (await import("@/messages/ru-RU.json")).default;
    case "tr-TR":
      return (await import("@/messages/tr-TR.json")).default;
    case "zh-CN":
      return (await import("@/messages/zh-CN.json")).default;
  }
}


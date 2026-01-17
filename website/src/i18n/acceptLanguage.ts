import { DEFAULT_LOCALE, type Locale } from "@/i18n/locales";

const LANGUAGE_TO_LOCALE: Record<string, Locale> = {
  en: "en-US",
  es: "es-ES",
  fr: "fr-FR",
  it: "it-IT",
  ja: "ja-JP",
  pt: "pt-BR",
  ru: "ru-RU",
  tr: "tr-TR",
  zh: "zh-CN"
};

function parseAcceptLanguage(header: string): string[] {
  // Minimal parsing: split by comma, ignore q-values, keep order.
  return header
    .split(",")
    .map((part) => part.trim().split(";")[0]?.trim())
    .filter((part): part is string => Boolean(part));
}

export function pickLocaleFromAcceptLanguage(header: string | null): Locale {
  if (!header) return DEFAULT_LOCALE;

  for (const tag of parseAcceptLanguage(header)) {
    const lower = tag.toLowerCase();
    const [lang] = lower.split("-");
    if (!lang) continue;
    const mapped = LANGUAGE_TO_LOCALE[lang];
    if (mapped) return mapped;
  }

  return DEFAULT_LOCALE;
}


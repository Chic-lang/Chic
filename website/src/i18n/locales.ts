export const SUPPORTED_LOCALES = [
  "en-US",
  "es-ES",
  "fr-FR",
  "it-IT",
  "ja-JP",
  "pt-BR",
  "ru-RU",
  "tr-TR",
  "zh-CN"
] as const;

export type Locale = (typeof SUPPORTED_LOCALES)[number];

export const DEFAULT_LOCALE: Locale = "en-US";

export function isLocale(value: string): value is Locale {
  // Keep this intentionally strict: only the supported list is accepted.
  return (SUPPORTED_LOCALES as readonly string[]).includes(value);
}


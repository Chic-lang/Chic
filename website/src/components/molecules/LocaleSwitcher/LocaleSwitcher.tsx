"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { SUPPORTED_LOCALES, type Locale } from "@/i18n/locales";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";
import styles from "./LocaleSwitcher.module.css";

const LOCALE_LABELS: Record<Locale, string> = {
  "en-US": "English (US)",
  "es-ES": "Español (ES)",
  "fr-FR": "Français (FR)",
  "it-IT": "Italiano (IT)",
  "ja-JP": "日本語 (JP)",
  "pt-BR": "Português (BR)",
  "ru-RU": "Русский (RU)",
  "tr-TR": "Türkçe (TR)",
  "zh-CN": "中文（简体）"
};

export function LocaleSwitcher({ locale }: { locale: Locale }) {
  const router = useRouter();
  const pathname = usePathname() ?? "/";
  const searchParams = useSearchParams();

  const { pathname: pathnameNoLocale } = stripLocaleFromPathname(pathname);
  const query = searchParams.toString();

  return (
    <div className={styles.root}>
      <label className={styles.label} htmlFor="locale-switcher">
        Language
      </label>
      <select
        id="locale-switcher"
        className={styles.select}
        value={locale}
        onChange={(e) => {
          const nextLocale = e.target.value as Locale;
          const nextPath = withLocale(nextLocale, pathnameNoLocale);
          router.push(query ? `${nextPath}?${query}` : nextPath);
        }}
      >
        {SUPPORTED_LOCALES.map((l) => (
          <option key={l} value={l}>
            {LOCALE_LABELS[l]}
          </option>
        ))}
      </select>
    </div>
  );
}


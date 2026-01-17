"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import type { Locale } from "@/i18n/locales";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";
import styles from "./LocaleSwitcher.module.css";

export type LocaleSwitcherOption = {
  locale: Locale;
  label: string;
};

export function LocaleSwitcher({
  locale,
  label,
  options
}: {
  locale: Locale;
  label: string;
  options: LocaleSwitcherOption[];
}) {
  const router = useRouter();
  const pathname = usePathname() ?? "/";
  const searchParams = useSearchParams();

  const { pathname: pathnameNoLocale } = stripLocaleFromPathname(pathname);
  const query = searchParams.toString();

  return (
    <div className={styles.root}>
      <label className={styles.label} htmlFor="locale-switcher">
        {label}
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
        {options.map((opt) => (
          <option key={opt.locale} value={opt.locale}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

import type { Metadata } from "next";
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, type Locale } from "@/i18n/locales";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";
import { getSiteUrl } from "@/lib/siteUrl";

export function canonicalUrl(locale: Locale, pathname: string): string {
  const { pathname: pathnameNoLocale } = stripLocaleFromPathname(pathname);
  return `${getSiteUrl()}${withLocale(locale, pathnameNoLocale)}`;
}

export function alternatesForPath(locale: Locale, pathname: string): Metadata["alternates"] {
  const { pathname: pathnameNoLocale } = stripLocaleFromPathname(pathname);

  const languages = Object.fromEntries(
    SUPPORTED_LOCALES.map((l) => [l, `${getSiteUrl()}${withLocale(l, pathnameNoLocale)}`])
  );
  languages["x-default"] = `${getSiteUrl()}${withLocale(DEFAULT_LOCALE, pathnameNoLocale)}`;

  return {
    canonical: `${getSiteUrl()}${withLocale(locale, pathnameNoLocale)}`,
    languages
  };
}


import { DEFAULT_LOCALE, isLocale, type Locale } from "@/i18n/locales";

export function withLocale(locale: Locale, pathname: string): string {
  const normalized = pathname.startsWith("/") ? pathname : `/${pathname}`;
  if (normalized === "/") return `/${locale}`;
  return `/${locale}${normalized}`;
}

export function stripLocaleFromPathname(pathname: string): { locale: Locale; pathname: string } {
  const normalized = pathname.startsWith("/") ? pathname : `/${pathname}`;
  const [_, first, ...rest] = normalized.split("/");

  if (first && isLocale(first)) {
    const remainder = `/${rest.join("/")}`;
    return { locale: first, pathname: remainder === "/" ? "/" : remainder.replace(/\/$/, "") };
  }

  return { locale: DEFAULT_LOCALE, pathname: normalized === "/" ? "/" : normalized.replace(/\/$/, "") };
}


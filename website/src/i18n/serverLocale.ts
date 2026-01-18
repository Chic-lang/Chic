import { notFound } from "next/navigation";
import { isLocale, type Locale } from "@/i18n/locales";

export async function getLocaleFromParams<T extends { locale: string }>(params: Promise<T>): Promise<Locale> {
  const { locale } = await params;
  if (!isLocale(locale)) return notFound();
  return locale;
}


"use client";

import { useEffect } from "react";
import type { Locale } from "@/i18n/locales";

export function HtmlLang({ locale }: { locale: Locale }) {
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return null;
}


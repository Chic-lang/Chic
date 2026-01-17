import type { Metadata } from "next";
import "@/app/globals.css";
import styles from "@/app/layout.module.css";
import { headers } from "next/headers";
import { DEFAULT_LOCALE, isLocale } from "@/i18n/locales";

export const metadata: Metadata = {
  title: {
    default: "Chic",
    template: "%s · Chic"
  },
  description: "Chic is an alpha AI-first programming language and toolchain."
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const localeHeader = (await headers()).get("x-chic-locale");
  const locale = localeHeader && isLocale(localeHeader) ? localeHeader : DEFAULT_LOCALE;

  return (
    <html lang={locale}>
      <body className={styles.shell}>
        {children}
      </body>
    </html>
  );
}

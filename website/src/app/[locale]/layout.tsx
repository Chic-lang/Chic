import styles from "@/app/layout.module.css";
import { notFound } from "next/navigation";
import { SiteFooter } from "@/components/organisms/SiteFooter/SiteFooter";
import { SiteHeader } from "@/components/organisms/SiteHeader/SiteHeader";
import { isLocale, SUPPORTED_LOCALES, type Locale } from "@/i18n/locales";

export function generateStaticParams() {
  return SUPPORTED_LOCALES.map((locale) => ({ locale }));
}

export default async function LocaleLayout({
  children,
  params
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale: localeRaw } = await params;
  if (!isLocale(localeRaw)) return notFound();

  const locale: Locale = localeRaw;

  return (
    <>
      <SiteHeader locale={locale} />
      <main id="main" className={styles.main}>
        <div className={styles.container}>{children}</div>
      </main>
      <SiteFooter locale={locale} />
    </>
  );
}


import type { Metadata } from "next";
import { HomeTemplate } from "@/components/templates/HomeTemplate/HomeTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { loadMessages } from "@/i18n/messages";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const tHome = await getTranslations({ locale, namespace: "pages.home" });
  const tSite = await getTranslations({ locale, namespace: "site" });

  const title = tHome("title");
  const description = tSite("description");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/")
    }
  };
}

export default async function Page({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const messages = await loadMessages(locale);
  const copy = (messages as any).pages.home;

  return <HomeTemplate locale={locale} copy={copy} />;
}

import type { Metadata } from "next";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.blog" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/blog"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/blog")
    }
  };
}

export default async function BlogIndexPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.blog" });
  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <BlogIndexTemplate locale={locale} page={1} />
    </SimplePageTemplate>
  );
}

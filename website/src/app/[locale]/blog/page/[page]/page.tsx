import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

function parsePage(value: string): number | null {
  if (!/^[0-9]+$/.test(value)) return null;
  const num = Number(value);
  if (!Number.isFinite(num) || num < 1) return null;
  return num;
}

export async function generateMetadata({
  params
}: {
  params: Promise<{ locale: string; page: string }>;
}): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.blog" });
  const { page } = await params;
  const pageNumber = parsePage(page);

  const title = t("title");
  const description = t("lede");
  const canonicalPath = pageNumber && pageNumber > 1 ? `/blog/page/${pageNumber}` : "/blog";

  return {
    title,
    description,
    alternates: alternatesForPath(locale, canonicalPath),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, canonicalPath)
    }
  };
}

export default async function BlogPage({ params }: { params: Promise<{ locale: string; page: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.blog" });
  const { page } = await params;
  const pageNumber = parsePage(page);
  if (!pageNumber) return notFound();

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <BlogIndexTemplate locale={locale} page={pageNumber} />
    </SimplePageTemplate>
  );
}

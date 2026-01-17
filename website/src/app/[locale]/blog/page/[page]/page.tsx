import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata: Metadata = { title: "Blog" };

function parsePage(value: string): number | null {
  if (!/^[0-9]+$/.test(value)) return null;
  const num = Number(value);
  if (!Number.isFinite(num) || num < 1) return null;
  return num;
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

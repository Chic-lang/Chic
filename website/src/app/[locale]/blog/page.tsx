import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata = { title: "Blog" };

export default async function BlogIndexPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.blog" });
  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <BlogIndexTemplate locale={locale} page={1} />
    </SimplePageTemplate>
  );
}

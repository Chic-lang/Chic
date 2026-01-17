import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";

export const metadata = { title: "Blog" };

export default async function BlogIndexPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  return (
    <SimplePageTemplate title="Blog" lede="Updates and roadmap notes as Chic evolves.">
      <BlogIndexTemplate locale={locale} page={1} />
    </SimplePageTemplate>
  );
}

import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { FallbackNotice } from "@/components/molecules/FallbackNotice/FallbackNotice";
import { Mdx } from "@/components/molecules/Mdx/Mdx";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getDocBySlug } from "@/lib/docs";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

const REPO = "https://github.com/Chic-lang/Chic";

export async function generateMetadata({
  params
}: {
  params: Promise<{ locale: string; slug: string[] }>;
}): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const { slug } = await params;
  const doc = getDocBySlug(locale, slug);
  if (!doc) return { title: "Docs" };
  return { title: doc.frontmatter.title, description: doc.frontmatter.description };
}

export default async function DocPage({ params }: { params: Promise<{ locale: string; slug: string[] }> }) {
  const locale = await getLocaleFromParams(params);
  const tDocs = await getTranslations({ locale, namespace: "pages.docs" });
  const tI18n = await getTranslations({ locale, namespace: "i18n" });
  const { slug } = await params;
  const doc = getDocBySlug(locale, slug);
  if (!doc) return notFound();

  return (
    <SimplePageTemplate title={doc.frontmatter.title} lede={doc.frontmatter.description}>
      <Prose>
        <p>
          {tDocs("sourceLabel")}{" "}
          <a href={`${REPO}/blob/main/${doc.sourcePath}`} target="_blank" rel="noreferrer">
            {doc.sourcePath}
          </a>
        </p>
        {doc.isFallback ? <FallbackNotice message={tI18n("fallbackNotice")} /> : null}
        <Mdx source={doc.content} locale={locale} />
        <p>
          <Link href={withLocale(locale, "/docs")}>{tDocs("backToDocs")}</Link>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

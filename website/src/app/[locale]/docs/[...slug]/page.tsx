import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { FallbackNotice } from "@/components/molecules/FallbackNotice/FallbackNotice";
import { Mdx } from "@/components/molecules/Mdx/Mdx";
import { RelatedLinks } from "@/components/molecules/RelatedLinks/RelatedLinks";
import { Prose } from "@/components/molecules/Prose/Prose";
import { ContactBlock } from "@/components/molecules/ContactBlock/ContactBlock";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getDocBySlug } from "@/lib/docs";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
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

  const pathname = `/docs/${slug.join("/")}`;
  const title = doc.frontmatter.title;
  const description = doc.frontmatter.description;

  return {
    title,
    description,
    alternates: alternatesForPath(locale, pathname),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, pathname)
    }
  };
}

export default async function DocPage({ params }: { params: Promise<{ locale: string; slug: string[] }> }) {
  const locale = await getLocaleFromParams(params);
  const tDocs = await getTranslations({ locale, namespace: "pages.docs" });
  const tI18n = await getTranslations({ locale, namespace: "i18n" });
  const tRelated = await getTranslations({ locale, namespace: "blocks.relatedLinks" });
  const { slug } = await params;
  const doc = getDocBySlug(locale, slug);
  if (!doc) return notFound();

  const relatedLinks = doc.frontmatter.relatedLinks ?? [];
  const showContactBlock = doc.frontmatter.contactBlock ?? true;

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
        <RelatedLinks locale={locale} title={tRelated("title")} links={relatedLinks} />
        {showContactBlock ? <ContactBlock locale={locale} /> : null}
        <p>
          <Link href={withLocale(locale, "/docs")}>{tDocs("backToDocs")}</Link>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

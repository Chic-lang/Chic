import Link from "next/link";
import type { Metadata } from "next";
import { listDocs } from "@/lib/docs";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.docs" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/docs"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/docs")
    }
  };
}

export default async function DocsLandingPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.docs" });
  const docs = listDocs(locale);

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <p>{t("body")}</p>
        <ul>
          {docs.map((doc) => (
            <li key={doc.slug.join("/")}>
              <Link href={withLocale(locale, `/docs/${doc.slug.join("/")}`)}>{doc.title}</Link>
              {doc.description ? ` — ${doc.description}` : null}
            </li>
          ))}
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}

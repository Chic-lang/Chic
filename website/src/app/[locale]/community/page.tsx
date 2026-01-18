import type { Metadata } from "next";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.community" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/community"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/community")
    }
  };
}

const REPO = "https://github.com/Chic-lang/Chic";

export default async function CommunityPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.community" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <p>{t("body")}</p>
        <ul>
          <li>
            <a href={`${REPO}/issues`} target="_blank" rel="noreferrer">
              {t("browseIssues")}
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/CONTRIBUTING.md`} target="_blank" rel="noreferrer">
              {t("contributingGuide")}
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/SUPPORT.md`} target="_blank" rel="noreferrer">
              {t("support")}
            </a>
          </li>
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}

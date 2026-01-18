import Link from "next/link";
import type { Metadata } from "next";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.learn" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/learn"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/learn")
    }
  };
}

export default async function LearnPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.learn" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <ul>
          <li>
            <Link href={withLocale(locale, "/docs/mission")}>{t("missionLink")}</Link>
          </li>
          <li>
            <Link href={withLocale(locale, "/docs/getting-started")}>{t("gettingStartedLink")}</Link>
          </li>
          <li>
            <Link href={withLocale(locale, "/docs/language/tour")}>{t("languageTourLink")}</Link>
          </li>
          <li>
            <a href="https://github.com/Chic-lang/Chic/blob/main/SPEC.md" target="_blank" rel="noreferrer">
              {t("specLink")}
            </a>
          </li>
        </ul>
        <p>{t("body")}</p>
      </Prose>
    </SimplePageTemplate>
  );
}

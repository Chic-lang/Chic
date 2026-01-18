import type { Metadata } from "next";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.tools" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/tools"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/tools")
    }
  };
}

const REPO = "https://github.com/Chic-lang/Chic";

export default async function ToolsPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.tools" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <h2>{t("cliTitle")}</h2>
        <p>{t("cliBody")}</p>
        <h2>{t("vscodeTitle")}</h2>
        <p>{t("vscodeBody")}</p>
        <p>
          {t("sourceLabel")}{" "}
          <a href={`${REPO}/tree/main/chic-vscode`} target="_blank" rel="noreferrer">
            chic-vscode/
          </a>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

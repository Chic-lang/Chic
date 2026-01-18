import type { Metadata } from "next";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.install" });

  const title = t("title");
  const description = t("lede");

  return {
    title,
    description,
    alternates: alternatesForPath(locale, "/install"),
    openGraph: {
      title,
      description,
      url: canonicalUrl(locale, "/install")
    }
  };
}

export default async function InstallPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.install" });

  return (
    <SimplePageTemplate
      title={t("title")}
      lede={t("lede")}
    >
      <Prose>
        <p>{t("alphaNotice")}</p>
        <h2>{t("buildCliTitle")}</h2>
        <pre>
          <code>{`cargo build --bin chic\n./target/debug/chic --help`}</code>
        </pre>
        <h2>{t("runBuildTitle")}</h2>
        <pre>
          <code>{`./target/debug/chic build`}</code>
        </pre>
        <h2>{t("createProjectTitle")}</h2>
        <pre>
          <code>{`./target/debug/chic init --template app-console --output ./hello\n./target/debug/chic build ./hello`}</code>
        </pre>
      </Prose>
    </SimplePageTemplate>
  );
}

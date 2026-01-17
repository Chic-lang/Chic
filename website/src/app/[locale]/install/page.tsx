import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata = { title: "Install" };

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

import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata = { title: "Tools" };

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

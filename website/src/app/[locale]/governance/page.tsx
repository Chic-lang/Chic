import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata = { title: "Governance" };

const REPO = "https://github.com/Chic-lang/Chic";

export default async function GovernancePage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.governance" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <p>{t("body")}</p>
        <ul>
          <li>
            <a href={`${REPO}/blob/main/README.md`} target="_blank" rel="noreferrer">
              {t("readme")}
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/docs/mission.md`} target="_blank" rel="noreferrer">
              {t("mission")}
            </a>
          </li>
          <li>
            <a href={`${REPO}/issues`} target="_blank" rel="noreferrer">
              {t("issues")}
            </a>
          </li>
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}

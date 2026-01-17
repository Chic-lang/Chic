import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export const metadata = { title: "Playground" };

export default async function PlaygroundPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const t = await getTranslations({ locale, namespace: "pages.playground" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <p>{t("body1")}</p>
        <p>{t("body2")}</p>
      </Prose>
    </SimplePageTemplate>
  );
}

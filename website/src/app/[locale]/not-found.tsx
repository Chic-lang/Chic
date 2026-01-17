import Link from "next/link";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { Button } from "@/components/atoms/Button/Button";
import { Prose } from "@/components/molecules/Prose/Prose";
import { headers } from "next/headers";
import { DEFAULT_LOCALE, isLocale, type Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

export default async function NotFound() {
  const localeHeader = (await headers()).get("x-chic-locale");
  const locale: Locale = localeHeader && isLocale(localeHeader) ? localeHeader : DEFAULT_LOCALE;
  const t = await getTranslations({ locale, namespace: "pages.notFound" });

  return (
    <SimplePageTemplate title={t("title")} lede={t("lede")}>
      <Prose>
        <p>{t("body")}</p>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          <Button href={withLocale(locale, "/")} variant="primary">
            {t("backToHome")}
          </Button>
          <Link href={withLocale(locale, "/docs")}>{t("goToDocs")}</Link>
        </div>
      </Prose>
    </SimplePageTemplate>
  );
}

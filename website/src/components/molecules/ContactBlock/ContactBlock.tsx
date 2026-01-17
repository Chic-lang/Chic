import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";
import { ContactBlockView } from "@/components/molecules/ContactBlock/ContactBlockView";

const REPO = "https://github.com/Chic-lang/Chic";

export async function ContactBlock({ locale }: { locale: Locale }) {
  const t = await getTranslations({ locale, namespace: "blocks.contact" });

  return (
    <ContactBlockView
      title={t("title")}
      body={t("body")}
      links={[
        { label: t("reportIssue"), href: `${REPO}/issues/new/choose`, external: true },
        { label: t("browseIssues"), href: `${REPO}/issues`, external: true },
        { label: t("contributingGuide"), href: `${REPO}/blob/main/CONTRIBUTING.md`, external: true },
        { label: t("community"), href: withLocale(locale, "/community") }
      ]}
    />
  );
}

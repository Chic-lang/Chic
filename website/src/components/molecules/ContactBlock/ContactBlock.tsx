import Link from "next/link";
import styles from "./ContactBlock.module.css";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

const REPO = "https://github.com/Chic-lang/Chic";

export async function ContactBlock({ locale }: { locale: Locale }) {
  const t = await getTranslations({ locale, namespace: "blocks.contact" });

  return (
    <aside className={styles.root} aria-labelledby="contact-block-title">
      <h2 id="contact-block-title" className={styles.title}>
        {t("title")}
      </h2>
      <p className={styles.body}>{t("body")}</p>
      <ul className={styles.actions}>
        <li>
          <a href={`${REPO}/issues/new/choose`} target="_blank" rel="noreferrer">
            {t("reportIssue")}
          </a>
        </li>
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
          <Link href={withLocale(locale, "/community")}>{t("community")}</Link>
        </li>
      </ul>
    </aside>
  );
}


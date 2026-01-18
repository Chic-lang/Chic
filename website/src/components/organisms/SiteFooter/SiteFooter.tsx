import styles from "./SiteFooter.module.css";
import Link from "next/link";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import { getTranslations } from "next-intl/server";

const REPO = "https://github.com/Chic-lang/Chic";

export async function SiteFooter({ locale }: { locale: Locale }) {
  const tFooter = await getTranslations({ locale, namespace: "footer" });
  const tNav = await getTranslations({ locale, namespace: "nav" });

  return (
    <footer className={styles.footer}>
      <div className={styles.inner}>
        <div>
          <h2 className={styles.colTitle}>{tFooter("getHelpTitle")}</h2>
          <Link className={styles.link} href={withLocale(locale, "/docs")}>
            {tNav("docs")}
          </Link>
          <Link className={styles.link} href={withLocale(locale, "/learn")}>
            {tNav("learn")}
          </Link>
          <Link className={styles.link} href={withLocale(locale, "/install")}>
            {tNav("install")}
          </Link>
        </div>
        <div>
          <h2 className={styles.colTitle}>{tFooter("projectTitle")}</h2>
          <a className={styles.link} href={REPO} target="_blank" rel="noreferrer">
            {tFooter("github")}
          </a>
          <Link className={styles.link} href={withLocale(locale, "/community")}>
            {tNav("community")}
          </Link>
          <Link className={styles.link} href={withLocale(locale, "/governance")}>
            {tNav("governance")}
          </Link>
        </div>
        <div>
          <h2 className={styles.colTitle}>{tFooter("policiesTitle")}</h2>
          <a className={styles.link} href={`${REPO}/blob/main/SECURITY.md`} target="_blank" rel="noreferrer">
            {tFooter("securityPolicy")}
          </a>
          <a
            className={styles.link}
            href={`${REPO}/blob/main/CODE_OF_CONDUCT.md`}
            target="_blank"
            rel="noreferrer"
          >
            {tFooter("codeOfConduct")}
          </a>
          <a className={styles.link} href={`${REPO}/blob/main/LICENSE`} target="_blank" rel="noreferrer">
            {tFooter("license")}
          </a>
        </div>
        <div className={styles.meta}>
          {tFooter("meta")}
        </div>
      </div>
    </footer>
  );
}

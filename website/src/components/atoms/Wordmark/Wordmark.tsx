import Image from "next/image";
import Link from "next/link";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./Wordmark.module.css";

export function Wordmark({ locale, name }: { locale: Locale; name: string }) {
  return (
    <Link className={styles.brand} href={withLocale(locale, "/")}>
      <Image className={styles.mark} src="/chick.svg" alt="" width={28} height={28} priority />
      <span className={styles.name}>{name}</span>
    </Link>
  );
}

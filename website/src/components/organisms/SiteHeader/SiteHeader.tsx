import { SkipLink } from "@/components/atoms/SkipLink/SkipLink";
import { Wordmark } from "@/components/atoms/Wordmark/Wordmark";
import { LocaleSwitcher } from "@/components/molecules/LocaleSwitcher/LocaleSwitcher";
import { NavLink } from "@/components/molecules/NavLink/NavLink";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./SiteHeader.module.css";

const NAV = [
  { href: "/install", label: "Install" },
  { href: "/learn", label: "Learn" },
  { href: "/playground", label: "Playground" },
  { href: "/tools", label: "Tools" },
  { href: "/governance", label: "Governance" },
  { href: "/community", label: "Community" },
  { href: "/blog", label: "Blog" },
  { href: "/docs", label: "Docs" }
] as const;

export function SiteHeader({ locale }: { locale: Locale }) {
  return (
    <header className={styles.header}>
      <SkipLink />
      <div className={styles.inner}>
        <Wordmark locale={locale} />
        <div className={styles.right}>
          <nav className={styles.nav} aria-label="Primary">
            {NAV.map((item) => (
              <NavLink key={item.href} href={withLocale(locale, item.href)}>
                {item.label}
              </NavLink>
            ))}
          </nav>
          <LocaleSwitcher locale={locale} />
        </div>
      </div>
    </header>
  );
}

import { SkipLink } from "@/components/atoms/SkipLink/SkipLink";
import { Wordmark } from "@/components/atoms/Wordmark/Wordmark";
import { LocaleSwitcher } from "@/components/molecules/LocaleSwitcher/LocaleSwitcher";
import { NavLink } from "@/components/molecules/NavLink/NavLink";
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, type Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./SiteHeader.module.css";
import { getTranslations } from "next-intl/server";
import { headers } from "next/headers";
import { hasBlogPostTranslation, hasDocTranslation } from "@/lib/contentAvailability";

const NAV = [
  { href: "/install", key: "install" },
  { href: "/learn", key: "learn" },
  { href: "/playground", key: "playground" },
  { href: "/tools", key: "tools" },
  { href: "/governance", key: "governance" },
  { href: "/community", key: "community" },
  { href: "/blog", key: "blog" },
  { href: "/docs", key: "docs" }
] as const;

export async function SiteHeader({ locale }: { locale: Locale }) {
  const tA11y = await getTranslations({ locale, namespace: "a11y" });
  const tI18n = await getTranslations({ locale, namespace: "i18n" });
  const tNav = await getTranslations({ locale, namespace: "nav" });
  const tLocaleNames = await getTranslations({ locale, namespace: "localeNames" });
  const tSite = await getTranslations({ locale, namespace: "site" });

  const pathnameNoLocale = (await headers()).get("x-chic-pathname-no-locale") ?? "/";
  const docSlug =
    pathnameNoLocale.startsWith("/docs/") && pathnameNoLocale !== "/docs"
      ? pathnameNoLocale.slice("/docs/".length).split("/").filter(Boolean)
      : null;
  const blogSlug =
    pathnameNoLocale.startsWith("/blog/") &&
    !pathnameNoLocale.startsWith("/blog/page/") &&
    pathnameNoLocale !== "/blog/rss.xml"
      ? pathnameNoLocale.slice("/blog/".length).split("/").filter(Boolean)[0] ?? null
      : null;

  const options = SUPPORTED_LOCALES.map((l) => {
    const baseLabel = tLocaleNames(l);
    if (l === DEFAULT_LOCALE) return { locale: l, label: baseLabel };

    const translated = docSlug ? hasDocTranslation(l, docSlug) : blogSlug ? hasBlogPostTranslation(l, blogSlug) : true;
    return {
      locale: l,
      label: translated ? baseLabel : `${baseLabel}${tI18n("fallbackOptionSuffix")}`
    };
  });

  return (
    <header className={styles.header}>
      <SkipLink label={tA11y("skipToContent")} />
      <div className={styles.inner}>
        <Wordmark locale={locale} name={tSite("name")} />
        <div className={styles.right}>
          <nav className={styles.nav} aria-label={tA11y("primaryNav")}>
            {NAV.map((item) => (
              <NavLink key={item.href} href={withLocale(locale, item.href)}>
                {tNav(item.key)}
              </NavLink>
            ))}
          </nav>
          <LocaleSwitcher
            locale={locale}
            label={tA11y("languageLabel")}
            options={options}
          />
        </div>
      </div>
    </header>
  );
}

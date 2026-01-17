import Link from "next/link";
import styles from "./RelatedLinks.module.css";
import type { Locale } from "@/i18n/locales";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";
import type { RelatedLink } from "@/lib/frontmatter";

function isExternalUrl(url: string): boolean {
  return /^https?:\/\//.test(url);
}

export function RelatedLinks({ locale, title, links }: { locale: Locale; title: string; links: RelatedLink[] }) {
  if (links.length === 0) return null;

  return (
    <aside className={styles.root} aria-labelledby="related-links-title">
      <h2 id="related-links-title" className={styles.title}>
        {title}
      </h2>
      <ul className={styles.list}>
        {links.map((link) => {
          const href = link.href;
          const isExternal = isExternalUrl(href);

          const content = (
            <>
              <div className={styles.itemTitle}>{link.title}</div>
              {link.description ? <p className={styles.itemDescription}>{link.description}</p> : null}
            </>
          );

          if (isExternal) {
            return (
              <li key={`${link.title}:${link.href}`}>
                <a href={href} target="_blank" rel="noreferrer">
                  {content}
                </a>
              </li>
            );
          }

          if (href.startsWith("#")) {
            return (
              <li key={`${link.title}:${link.href}`}>
                <a href={href}>{content}</a>
              </li>
            );
          }

          if (href.startsWith("/")) {
            const { pathname } = stripLocaleFromPathname(href);
            return (
              <li key={`${link.title}:${link.href}`}>
                <Link href={withLocale(locale, pathname)}>{content}</Link>
              </li>
            );
          }

          return (
            <li key={`${link.title}:${link.href}`}>
              <a href={href}>{content}</a>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}


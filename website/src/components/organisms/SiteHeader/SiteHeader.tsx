import { SkipLink } from "@/components/atoms/SkipLink/SkipLink";
import { Wordmark } from "@/components/atoms/Wordmark/Wordmark";
import { NavLink } from "@/components/molecules/NavLink/NavLink";
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

export function SiteHeader() {
  return (
    <header className={styles.header}>
      <SkipLink />
      <div className={styles.inner}>
        <Wordmark />
        <nav className={styles.nav} aria-label="Primary">
          {NAV.map((item) => (
            <NavLink key={item.href} href={item.href}>
              {item.label}
            </NavLink>
          ))}
        </nav>
      </div>
    </header>
  );
}


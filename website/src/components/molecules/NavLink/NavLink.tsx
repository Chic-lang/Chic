import Link from "next/link";
import styles from "./NavLink.module.css";

export function NavLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <Link className={styles.link} href={href}>
      {children}
    </Link>
  );
}


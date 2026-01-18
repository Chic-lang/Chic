import Link from "next/link";
import styles from "./ContactBlock.module.css";

export type ContactBlockLink = {
  label: string;
  href: string;
  external?: boolean;
};

export function ContactBlockView({
  title,
  body,
  links
}: {
  title: string;
  body: string;
  links: ContactBlockLink[];
}) {
  return (
    <aside className={styles.root} aria-labelledby="contact-block-title">
      <h2 id="contact-block-title" className={styles.title}>
        {title}
      </h2>
      <p className={styles.body}>{body}</p>
      <ul className={styles.actions}>
        {links.map((link) => (
          <li key={`${link.label}:${link.href}`}>
            {link.external ? (
              <a href={link.href} target="_blank" rel="noreferrer">
                {link.label}
              </a>
            ) : (
              <Link href={link.href}>{link.label}</Link>
            )}
          </li>
        ))}
      </ul>
    </aside>
  );
}


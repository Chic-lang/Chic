import styles from "./SiteFooter.module.css";
import Link from "next/link";

const REPO = "https://github.com/Chic-lang/Chic";

export function SiteFooter() {
  return (
    <footer className={styles.footer}>
      <div className={styles.inner}>
        <div>
          <h2 className={styles.colTitle}>Get help</h2>
          <Link className={styles.link} href="/docs">
            Docs
          </Link>
          <Link className={styles.link} href="/learn">
            Learn
          </Link>
          <Link className={styles.link} href="/install">
            Install
          </Link>
        </div>
        <div>
          <h2 className={styles.colTitle}>Project</h2>
          <a className={styles.link} href={REPO} target="_blank" rel="noreferrer">
            GitHub
          </a>
          <Link className={styles.link} href="/community">
            Community
          </Link>
          <Link className={styles.link} href="/governance">
            Governance
          </Link>
        </div>
        <div>
          <h2 className={styles.colTitle}>Policies</h2>
          <a className={styles.link} href={`${REPO}/blob/main/SECURITY.md`} target="_blank" rel="noreferrer">
            Security policy
          </a>
          <a
            className={styles.link}
            href={`${REPO}/blob/main/CODE_OF_CONDUCT.md`}
            target="_blank"
            rel="noreferrer"
          >
            Code of conduct
          </a>
          <a className={styles.link} href={`${REPO}/blob/main/LICENSE`} target="_blank" rel="noreferrer">
            License
          </a>
        </div>
        <div className={styles.meta}>
          Chic is an alpha project. Content is sourced from the Chic monorepo docs and may change rapidly.
        </div>
      </div>
    </footer>
  );
}

import styles from "./SkipLink.module.css";

export function SkipLink({ label }: { label: string }) {
  return (
    <a className={styles.skip} href="#main">
      {label}
    </a>
  );
}

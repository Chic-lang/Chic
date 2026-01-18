import styles from "./InlineCode.module.css";

export function InlineCode({ children }: { children: React.ReactNode }) {
  return <code className={styles.code}>{children}</code>;
}


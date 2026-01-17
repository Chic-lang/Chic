import styles from "./FallbackNotice.module.css";

export function FallbackNotice({ message }: { message: string }) {
  return (
    <aside className={styles.notice} role="status" aria-live="polite">
      <p className={styles.text}>{message}</p>
    </aside>
  );
}


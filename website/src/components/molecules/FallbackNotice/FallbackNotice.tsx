import styles from "./FallbackNotice.module.css";

export function FallbackNotice({ message }: { message: string }) {
  return (
    <div className={styles.notice} role="status" aria-live="polite">
      <p className={styles.text}>{message}</p>
    </div>
  );
}

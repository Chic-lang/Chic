import styles from "./SimplePageTemplate.module.css";

export function SimplePageTemplate({
  title,
  lede,
  children
}: {
  title: string;
  lede?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <header className={styles.header}>
        <h1 className={styles.title}>{title}</h1>
        {lede ? <p className={styles.lede}>{lede}</p> : null}
      </header>
      <div className={styles.content}>{children}</div>
    </div>
  );
}


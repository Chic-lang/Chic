import Image from "next/image";
import Link from "next/link";
import styles from "./Wordmark.module.css";

export function Wordmark() {
  return (
    <Link className={styles.brand} href="/">
      <Image className={styles.mark} src="/chick.svg" alt="" width={28} height={28} priority />
      <span className={styles.name}>Chic</span>
    </Link>
  );
}


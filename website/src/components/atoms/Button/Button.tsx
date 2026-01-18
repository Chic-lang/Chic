import Link from "next/link";
import styles from "./Button.module.css";

type Variant = "primary" | "secondary";

type ButtonProps =
  | { href: string; children: React.ReactNode; variant?: Variant; external?: boolean }
  | { onClick: () => void; children: React.ReactNode; variant?: Variant };

export function Button(props: ButtonProps) {
  const variant = props.variant ?? "secondary";
  const className = `${styles.button} ${variant === "primary" ? styles.primary : styles.secondary}`;

  if ("href" in props) {
    if (props.external) {
      return (
        <a className={className} href={props.href} target="_blank" rel="noreferrer">
          {props.children}
        </a>
      );
    }

    return (
      <Link className={className} href={props.href}>
        {props.children}
      </Link>
    );
  }

  return (
    <button className={className} type="button" onClick={props.onClick}>
      {props.children}
    </button>
  );
}


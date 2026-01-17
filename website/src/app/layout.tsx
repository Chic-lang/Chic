import type { Metadata } from "next";
import "@/app/globals.css";
import styles from "@/app/layout.module.css";
import { SiteFooter } from "@/components/organisms/SiteFooter/SiteFooter";
import { SiteHeader } from "@/components/organisms/SiteHeader/SiteHeader";

export const metadata: Metadata = {
  title: {
    default: "Chic",
    template: "%s · Chic"
  },
  description: "Chic is an alpha AI-first programming language and toolchain."
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={styles.shell}>
        <SiteHeader />
        <main id="main" className={styles.main}>
          <div className={styles.container}>{children}</div>
        </main>
        <SiteFooter />
      </body>
    </html>
  );
}


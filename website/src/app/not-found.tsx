import Link from "next/link";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { Prose } from "@/components/molecules/Prose/Prose";
import { Button } from "@/components/atoms/Button/Button";
import { DEFAULT_LOCALE } from "@/i18n/locales";

export default function NotFound() {
  return (
    <SimplePageTemplate title="Page not found" lede="That page doesn’t exist (or moved).">
      <Prose>
        <p>This page is only served for non-locale routes. Try the site home instead.</p>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          <Button href={`/${DEFAULT_LOCALE}`} variant="primary">
            Back to home
          </Button>
          <Link href={`/${DEFAULT_LOCALE}/docs`}>Go to docs</Link>
        </div>
      </Prose>
    </SimplePageTemplate>
  );
}


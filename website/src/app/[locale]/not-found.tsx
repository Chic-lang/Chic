import Link from "next/link";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { Button } from "@/components/atoms/Button/Button";
import { Prose } from "@/components/molecules/Prose/Prose";
import { headers } from "next/headers";
import { DEFAULT_LOCALE, isLocale, type Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";

export default async function NotFound() {
  const localeHeader = (await headers()).get("x-chic-locale");
  const locale: Locale = localeHeader && isLocale(localeHeader) ? localeHeader : DEFAULT_LOCALE;

  return (
    <SimplePageTemplate title="Page not found" lede="That page doesn’t exist (or moved).">
      <Prose>
        <p>
          If you expected something here, please open an issue with the broken link and what you were trying to
          reach.
        </p>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          <Button href={withLocale(locale, "/")} variant="primary">
            Back to home
          </Button>
          <Link href={withLocale(locale, "/docs")}>Go to docs</Link>
        </div>
      </Prose>
    </SimplePageTemplate>
  );
}

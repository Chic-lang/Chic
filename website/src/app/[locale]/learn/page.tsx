import Link from "next/link";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { withLocale } from "@/i18n/paths";

export const metadata = { title: "Learn" };

export default async function LearnPage({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);

  return (
    <SimplePageTemplate title="Learn" lede="Start with the docs that define Chic’s goals, workflow, and language tour.">
      <Prose>
        <ul>
          <li>
            <Link href={withLocale(locale, "/docs/mission")}>Mission statement</Link>
          </li>
          <li>
            <Link href={withLocale(locale, "/docs/getting-started")}>Getting started</Link>
          </li>
          <li>
            <Link href={withLocale(locale, "/docs/language/tour")}>Language tour</Link>
          </li>
          <li>
            <a href="https://github.com/Chic-lang/Chic/blob/main/SPEC.md" target="_blank" rel="noreferrer">
              Specification (SPEC.md)
            </a>
          </li>
        </ul>
        <p>
          Chic is designed around deterministic workflows and AI-first feedback loops. If you’re new, start with the
          mission and getting-started docs, then move to the language tour.
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

import Link from "next/link";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Learn" };

export default function LearnPage() {
  return (
    <SimplePageTemplate title="Learn" lede="Start with the docs that define Chic’s goals, workflow, and language tour.">
      <Prose>
        <ul>
          <li>
            <Link href="/docs/mission">Mission statement</Link>
          </li>
          <li>
            <Link href="/docs/getting-started">Getting started</Link>
          </li>
          <li>
            <Link href="/docs/language/tour">Language tour</Link>
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


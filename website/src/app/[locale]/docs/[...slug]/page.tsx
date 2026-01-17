import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { Markdown } from "@/components/molecules/Markdown/Markdown";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { findDocBySlug, readDocMarkdown } from "@/lib/docs";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { withLocale } from "@/i18n/paths";

const REPO = "https://github.com/Chic-lang/Chic";

export async function generateMetadata({
  params
}: {
  params: Promise<{ locale: string; slug: string[] }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const doc = findDocBySlug(slug);
  if (!doc) return { title: "Docs" };
  return { title: doc.title, description: doc.description };
}

export default async function DocPage({ params }: { params: Promise<{ locale: string; slug: string[] }> }) {
  const locale = await getLocaleFromParams(params);
  const { slug } = await params;
  const doc = findDocBySlug(slug);
  if (!doc) return notFound();

  const markdown = readDocMarkdown(doc);

  return (
    <SimplePageTemplate title={doc.title} lede={doc.description}>
      <Prose>
        <p>
          Source:{" "}
          <a href={`${REPO}/blob/main/${doc.sourcePath}`} target="_blank" rel="noreferrer">
            {doc.sourcePath}
          </a>
        </p>
        <Markdown markdown={markdown} />
        <p>
          <Link href={withLocale(locale, "/docs")}>Back to docs</Link>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

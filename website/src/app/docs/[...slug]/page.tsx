import Link from "next/link";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { Markdown } from "@/components/molecules/Markdown/Markdown";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { findDocBySlug, readDocMarkdown } from "@/lib/docs";

const REPO = "https://github.com/Chic-lang/Chic";

export function generateMetadata({ params }: { params: { slug: string[] } }): Metadata {
  const doc = findDocBySlug(params.slug);
  if (!doc) return { title: "Docs" };
  return { title: doc.title, description: doc.description };
}

export default function DocPage({ params }: { params: { slug: string[] } }) {
  const doc = findDocBySlug(params.slug);
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
          <Link href="/docs">Back to docs</Link>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}


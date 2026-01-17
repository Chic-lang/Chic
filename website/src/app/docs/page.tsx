import Link from "next/link";
import { listDocs } from "@/lib/docs";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Docs" };

export default function DocsLandingPage() {
  const docs = listDocs();

  return (
    <SimplePageTemplate title="Docs" lede="Curated documentation rendered from this repo’s markdown files.">
      <Prose>
        <p>
          This is intentionally a curated subset for v1. Each page links back to its source file in the Chic monorepo.
        </p>
        <ul>
          {docs.map((doc) => (
            <li key={doc.slug.join("/")}>
              <Link href={`/docs/${doc.slug.join("/")}`}>{doc.title}</Link>
              {doc.description ? ` — ${doc.description}` : null}
            </li>
          ))}
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}


import type { DocEntry } from "@/content/docs";
import { DOCS } from "@/content/docs";
import { readWorkspaceTextFile } from "@/lib/workspace";

export function listDocs(): DocEntry[] {
  return DOCS;
}

export function findDocBySlug(slug: string[]): DocEntry | undefined {
  return DOCS.find((doc) => doc.slug.join("/") === slug.join("/"));
}

export function readDocMarkdown(doc: DocEntry): string {
  return readWorkspaceTextFile(doc.sourcePath);
}


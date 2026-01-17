import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";

export const metadata: Metadata = { title: "Blog" };

function parsePage(value: string): number | null {
  if (!/^[0-9]+$/.test(value)) return null;
  const num = Number(value);
  if (!Number.isFinite(num) || num < 1) return null;
  return num;
}

export default async function BlogPage({ params }: { params: Promise<{ page: string }> }) {
  const { page } = await params;
  const pageNumber = parsePage(page);
  if (!pageNumber) return notFound();

  return (
    <SimplePageTemplate title="Blog" lede="Updates and roadmap notes as Chic evolves.">
      <BlogIndexTemplate page={pageNumber} />
    </SimplePageTemplate>
  );
}


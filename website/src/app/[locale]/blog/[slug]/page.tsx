import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { Markdown } from "@/components/molecules/Markdown/Markdown";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getBlogPostBySlug } from "@/lib/blog";
import { getLocaleFromParams } from "@/i18n/serverLocale";

export async function generateMetadata({
  params
}: {
  params: Promise<{ locale: string; slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = getBlogPostBySlug(slug);
  if (!post) return { title: "Blog" };

  return {
    title: post.frontmatter.title,
    description: post.frontmatter.description
  };
}

export default async function BlogPostPage({ params }: { params: Promise<{ locale: string; slug: string }> }) {
  await getLocaleFromParams(params);
  const { slug } = await params;
  const post = getBlogPostBySlug(slug);
  if (!post) return notFound();

  return (
    <SimplePageTemplate title={post.frontmatter.title} lede={post.frontmatter.description}>
      <Prose>
        <p>
          <time dateTime={post.frontmatter.date}>{post.frontmatter.date}</time>
          {post.frontmatter.author ? ` · ${post.frontmatter.author}` : null}
        </p>
        <Markdown markdown={post.content} />
      </Prose>
    </SimplePageTemplate>
  );
}

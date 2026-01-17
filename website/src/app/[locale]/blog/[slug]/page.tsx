import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { FallbackNotice } from "@/components/molecules/FallbackNotice/FallbackNotice";
import { Mdx } from "@/components/molecules/Mdx/Mdx";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { getBlogPostBySlug } from "@/lib/blog";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { getTranslations } from "next-intl/server";

export async function generateMetadata({
  params
}: {
  params: Promise<{ locale: string; slug: string }>;
}): Promise<Metadata> {
  const locale = await getLocaleFromParams(params);
  const { slug } = await params;
  const post = getBlogPostBySlug(locale, slug);
  if (!post) return { title: "Blog" };

  return {
    title: post.frontmatter.title,
    description: post.frontmatter.description
  };
}

export default async function BlogPostPage({ params }: { params: Promise<{ locale: string; slug: string }> }) {
  const locale = await getLocaleFromParams(params);
  const tI18n = await getTranslations({ locale, namespace: "i18n" });
  const { slug } = await params;
  const post = getBlogPostBySlug(locale, slug);
  if (!post) return notFound();

  return (
    <SimplePageTemplate title={post.frontmatter.title} lede={post.frontmatter.description}>
      <Prose>
        {post.isFallback ? <FallbackNotice message={tI18n("fallbackNotice")} /> : null}
        <p>
          <time dateTime={post.frontmatter.date}>{post.frontmatter.date}</time>
          {post.frontmatter.author ? ` · ${post.frontmatter.author}` : null}
        </p>
        <Mdx source={post.content} locale={locale} />
      </Prose>
    </SimplePageTemplate>
  );
}

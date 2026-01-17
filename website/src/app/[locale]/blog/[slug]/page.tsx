import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { FallbackNotice } from "@/components/molecules/FallbackNotice/FallbackNotice";
import { Mdx } from "@/components/molecules/Mdx/Mdx";
import { Prose } from "@/components/molecules/Prose/Prose";
import { RelatedLinks } from "@/components/molecules/RelatedLinks/RelatedLinks";
import { ContactBlock } from "@/components/molecules/ContactBlock/ContactBlock";
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
  const tRelated = await getTranslations({ locale, namespace: "blocks.relatedLinks" });
  const { slug } = await params;
  const post = getBlogPostBySlug(locale, slug);
  if (!post) return notFound();

  const relatedLinks = post.frontmatter.relatedLinks ?? [];
  const showContactBlock = post.frontmatter.contactBlock ?? true;

  return (
    <SimplePageTemplate title={post.frontmatter.title} lede={post.frontmatter.description}>
      <Prose>
        {post.isFallback ? <FallbackNotice message={tI18n("fallbackNotice")} /> : null}
        <p>
          <time dateTime={post.frontmatter.date}>{post.frontmatter.date}</time>
          {post.frontmatter.author ? ` · ${post.frontmatter.author}` : null}
        </p>
        <Mdx source={post.content} locale={locale} />
        <RelatedLinks locale={locale} title={tRelated("title")} links={relatedLinks} />
        {showContactBlock ? <ContactBlock locale={locale} /> : null}
      </Prose>
    </SimplePageTemplate>
  );
}

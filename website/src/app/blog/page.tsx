import Link from "next/link";
import { listAllBlogPosts } from "@/lib/blog";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Blog" };

const PAGE_SIZE = 10;

export default function BlogIndexPage() {
  const posts = listAllBlogPosts();
  const pagePosts = posts.slice(0, PAGE_SIZE);

  return (
    <SimplePageTemplate title="Blog" lede="Updates and roadmap notes as Chic evolves.">
      <Prose>
        <ul>
          {pagePosts.map((post) => (
            <li key={post.slug}>
              <Link href={`/blog/${post.slug}`}>{post.frontmatter.title}</Link> —{" "}
              <time dateTime={post.frontmatter.date}>{post.frontmatter.date}</time>
            </li>
          ))}
        </ul>
        <p>
          <a href="/blog/rss.xml">RSS</a>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}


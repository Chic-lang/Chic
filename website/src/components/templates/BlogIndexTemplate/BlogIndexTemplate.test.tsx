import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("next-intl/server", () => ({
  getTranslations: async ({ namespace }: { namespace: string }) => {
    return (key: string, values?: Record<string, unknown>) => {
      if (namespace === "pages.blog" && key === "pageOf") {
        return `page ${values?.page} of ${values?.total}`;
      }
      return key;
    };
  }
}));

let postCount = 21;
vi.mock("@/lib/blog", () => ({
  listAllBlogPosts: () =>
    Array.from({ length: postCount }, (_, i) => ({
      slug: `post-${i + 1}`,
      frontmatter: { title: `Post ${i + 1}`, date: "2026-01-01" }
    }))
}));

import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";

describe("<BlogIndexTemplate />", () => {
  it("renders pager links with correct routing rules", async () => {
    postCount = 21;
    const element = await BlogIndexTemplate({ locale: "en-US", page: 2 });
    render(element);

    expect(screen.getByText("page 2 of 3")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "previous" })).toHaveAttribute("href", "/en-US/blog");
    expect(screen.getByRole("link", { name: "next" })).toHaveAttribute("href", "/en-US/blog/page/3");
  });

  it("renders a null prev link on the first page and null next link on the last page", async () => {
    postCount = 21;
    const first = await BlogIndexTemplate({ locale: "en-US", page: 1 });
    render(first);
    expect(screen.getByText("page 1 of 3")).toBeInTheDocument();
    expect(screen.getByText("previous")).toHaveAttribute("aria-hidden", "true");
    expect(screen.getByRole("link", { name: "next" })).toHaveAttribute("href", "/en-US/blog/page/2");
  });

  it("hides the pager entirely when there is only one page", async () => {
    postCount = 1;
    const element = await BlogIndexTemplate({ locale: "en-US", page: 1 });
    const { container } = render(element);
    expect(container.querySelector("nav[aria-label='paginationNav']")).toBeNull();
  });

  it("uses /blog/page/N for prev on page > 2 and renders a disabled next on the last page", async () => {
    postCount = 21;
    const element = await BlogIndexTemplate({ locale: "en-US", page: 3 });
    render(element);

    expect(screen.getByText("page 3 of 3")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "previous" })).toHaveAttribute("href", "/en-US/blog/page/2");
    expect(screen.getByText("next")).toHaveAttribute("aria-hidden", "true");
  });
});

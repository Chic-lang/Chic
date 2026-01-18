import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("next-mdx-remote/rsc", () => ({
  MDXRemote: ({ components }: { components: any }) => (
    <div>
      {components.h1({ children: "H1" })}
      {components.h2({ children: "H2" })}
      {components.h3({ children: "H3" })}
      {components.a({ href: "https://example.com", children: "external" })}
      {components.a({ children: "nohref" })}
      {components.a({ href: "#hash", children: "hash" })}
      {components.a({ href: "/docs/mission", children: "internal-abs" })}
      {components.a({ href: "docs/mission", children: "internal-rel" })}
    </div>
  )
}));

import { Mdx } from "@/components/molecules/Mdx/Mdx";

describe("<Mdx />", () => {
  it("localizes absolute internal links and keeps external links external", () => {
    render(<Mdx source={"# X"} locale="ja-JP" />);

    expect(screen.getByRole("heading", { level: 2, name: "H1" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "H2" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 4, name: "H3" })).toBeInTheDocument();

    const external = screen.getByRole("link", { name: "external" });
    expect(external).toHaveAttribute("href", "https://example.com");
    expect(external).toHaveAttribute("target", "_blank");

    expect(screen.getByRole("link", { name: "nohref" })).toHaveAttribute("href", "#");
    expect(screen.getByRole("link", { name: "hash" })).toHaveAttribute("href", "#hash");
    expect(screen.getByRole("link", { name: "internal-abs" })).toHaveAttribute("href", "/ja-JP/docs/mission");
    expect(screen.getByRole("link", { name: "internal-rel" })).toHaveAttribute("href", "docs/mission");
  });

  it("does not localize internal links when locale is not provided", () => {
    render(<Mdx source={"# X"} />);
    expect(screen.getByRole("link", { name: "internal-abs" })).toHaveAttribute("href", "/docs/mission");
  });
});

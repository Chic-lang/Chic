import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Markdown } from "@/components/molecules/Markdown/Markdown";

describe("<Markdown />", () => {
  it("demotes headings and handles external links safely", () => {
    render(<Markdown markdown={"# Title\n\n## H2\n\n### H3\n\n[ext](https://example.com)\n\n[int](/docs)\n\n[empty]()"} />);

    // h1 becomes h2
    expect(screen.getByRole("heading", { level: 2, name: "Title" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "H2" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 4, name: "H3" })).toBeInTheDocument();

    const ext = screen.getByRole("link", { name: "ext" });
    expect(ext).toHaveAttribute("target", "_blank");
    expect(ext).toHaveAttribute("rel", "noreferrer");

    const internal = screen.getByRole("link", { name: "int" });
    expect(internal).toHaveAttribute("href", "/docs");
    expect(internal).not.toHaveAttribute("target");

    expect(screen.getByRole("link", { name: "empty" })).toHaveAttribute("href", "#");
  });
});

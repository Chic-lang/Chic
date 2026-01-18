import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { RelatedLinks } from "@/components/molecules/RelatedLinks/RelatedLinks";

describe("<RelatedLinks />", () => {
  it("renders nothing for empty links", () => {
    const { container } = render(<RelatedLinks locale="en-US" title="Related" links={[]} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders external, hash, and localized internal links", () => {
    render(
      <RelatedLinks
        locale="fr-FR"
        title="Next steps"
        links={[
          { title: "External", href: "https://example.com", description: "desc" },
          { title: "Hash", href: "#section" },
          { title: "Internal", href: "/docs/mission" },
          { title: "Relative", href: "docs/mission" }
        ]}
      />
    );

    const external = screen.getByRole("link", { name: /External/ });
    expect(external).toHaveAttribute("target", "_blank");
    expect(screen.getByText("desc")).toBeInTheDocument();

    expect(screen.getByRole("link", { name: "Hash" })).toHaveAttribute("href", "#section");
    expect(screen.getByRole("link", { name: "Internal" })).toHaveAttribute("href", "/fr-FR/docs/mission");
    expect(screen.getByRole("link", { name: "Relative" })).toHaveAttribute("href", "docs/mission");
  });
});

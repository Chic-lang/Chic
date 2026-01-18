import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

describe("<SimplePageTemplate />", () => {
  it("renders optional lede when provided", () => {
    render(
      <SimplePageTemplate title="Title" lede="Lede">
        <div>Body</div>
      </SimplePageTemplate>
    );
    expect(screen.getByRole("heading", { level: 1, name: "Title" })).toBeInTheDocument();
    expect(screen.getByText("Lede")).toBeInTheDocument();
  });

  it("omits lede when not provided", () => {
    const { container } = render(
      <SimplePageTemplate title="Title">
        <div>Body</div>
      </SimplePageTemplate>
    );
    expect(container.textContent).not.toContain("Lede");
  });
});


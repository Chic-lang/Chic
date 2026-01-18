import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { NavLink } from "@/components/molecules/NavLink/NavLink";

describe("<NavLink />", () => {
  it("renders a link", () => {
    render(<NavLink href="/en-US/docs">Docs</NavLink>);
    expect(screen.getByRole("link", { name: "Docs" })).toHaveAttribute("href", "/en-US/docs");
  });
});


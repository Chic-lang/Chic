import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { SkipLink } from "@/components/atoms/SkipLink/SkipLink";

describe("<SkipLink />", () => {
  it("links to #main", () => {
    render(<SkipLink label="Skip" />);
    expect(screen.getByRole("link", { name: "Skip" })).toHaveAttribute("href", "#main");
  });
});


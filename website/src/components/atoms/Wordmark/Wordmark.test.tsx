import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Wordmark } from "@/components/atoms/Wordmark/Wordmark";

describe("<Wordmark />", () => {
  it("links to the locale root", () => {
    render(<Wordmark locale="en-US" name="Chic" />);
    expect(screen.getByRole("link", { name: "Chic" })).toHaveAttribute("href", "/en-US");
  });
});


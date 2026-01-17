import { render, screen } from "@testing-library/react";
import { HomeTemplate } from "./HomeTemplate";
import { describe, expect, it } from "vitest";

describe("HomeTemplate", () => {
  it("renders the hero heading", () => {
    render(<HomeTemplate />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Chic is an alpha");
  });
});

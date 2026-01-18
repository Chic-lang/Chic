import { render, screen } from "@testing-library/react";
import { HomeTemplate } from "./HomeTemplate";
import { describe, expect, it } from "vitest";
import enMessages from "@/messages/en-US.json";

describe("HomeTemplate", () => {
  it("renders the hero heading", () => {
    render(<HomeTemplate locale="en-US" copy={enMessages.pages.home} />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Chic is an alpha");
  });
});

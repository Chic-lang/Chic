import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("next-intl/server", () => ({
  getTranslations: async () => (key: string) => key
}));

import { ContactBlock } from "@/components/molecules/ContactBlock/ContactBlock";

describe("<ContactBlock />", () => {
  it("renders localized links with a consistent structure", async () => {
    const element = await ContactBlock({ locale: "en-US" });
    render(element);

    expect(screen.getByRole("heading", { level: 2, name: "title" })).toBeInTheDocument();
    expect(screen.getByText("body")).toBeInTheDocument();

    expect(screen.getByRole("link", { name: "community" })).toHaveAttribute("href", "/en-US/community");
    expect(screen.getByRole("link", { name: "reportIssue" })).toHaveAttribute("target", "_blank");
  });
});


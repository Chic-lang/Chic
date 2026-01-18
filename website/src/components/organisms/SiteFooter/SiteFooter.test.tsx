import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("next-intl/server", () => ({
  getTranslations: async () => (key: string) => key
}));

import { SiteFooter } from "@/components/organisms/SiteFooter/SiteFooter";

describe("<SiteFooter />", () => {
  it("renders footer links localized to the current locale", async () => {
    const element = await SiteFooter({ locale: "en-US" });
    render(element);
    expect(screen.getByRole("heading", { level: 2, name: "getHelpTitle" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "docs" })).toHaveAttribute("href", "/en-US/docs");
    expect(screen.getByRole("link", { name: "learn" })).toHaveAttribute("href", "/en-US/learn");
  });
});


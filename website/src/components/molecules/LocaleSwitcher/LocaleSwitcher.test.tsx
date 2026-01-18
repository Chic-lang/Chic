import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const push = vi.fn();
let pathname: string | null = "/en-US/docs/mission";
let searchParams: URLSearchParams = new URLSearchParams("q=1");

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push }),
  usePathname: () => pathname,
  useSearchParams: () => searchParams
}));

import { LocaleSwitcher } from "@/components/molecules/LocaleSwitcher/LocaleSwitcher";

describe("<LocaleSwitcher />", () => {
  beforeEach(() => {
    pathname = "/en-US/docs/mission";
    searchParams = new URLSearchParams("q=1");
    push.mockClear();
  });

  it("preserves the current path and query when switching locales", () => {
    render(
      <LocaleSwitcher
        locale="en-US"
        label="Language"
        options={[
          { locale: "en-US", label: "English" },
          { locale: "ja-JP", label: "日本語" }
        ]}
      />
    );

    fireEvent.change(screen.getByLabelText("Language"), { target: { value: "ja-JP" } });
    expect(push).toHaveBeenCalledWith("/ja-JP/docs/mission?q=1");
  });

  it("falls back to / when pathname is null", () => {
    pathname = null;
    render(
      <LocaleSwitcher
        locale="en-US"
        label="Language"
        options={[
          { locale: "en-US", label: "English" },
          { locale: "fr-FR", label: "Français" }
        ]}
      />
    );

    fireEvent.change(screen.getByLabelText("Language"), { target: { value: "fr-FR" } });
    expect(push).toHaveBeenCalledWith("/fr-FR?q=1");
  });

  it("handles empty query strings", () => {
    searchParams = new URLSearchParams();
    render(
      <LocaleSwitcher
        locale="en-US"
        label="Language"
        options={[
          { locale: "en-US", label: "English" },
          { locale: "fr-FR", label: "Français" }
        ]}
      />
    );

    fireEvent.change(screen.getByLabelText("Language"), { target: { value: "fr-FR" } });
    expect(push).toHaveBeenCalledWith("/fr-FR/docs/mission");
  });
});

import "@testing-library/jest-dom/vitest";
import React from "react";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
});

vi.mock("next/image", () => ({
  __esModule: true,
  default: ({ priority: _priority, ...props }: Record<string, unknown>) => React.createElement("img", props)
}));

vi.mock("next/link", () => ({
  __esModule: true,
  default: ({
    href,
    children,
    ...rest
  }: {
    href: string | { pathname?: string };
    children: React.ReactNode;
  }) => {
    const resolved = typeof href === "string" ? href : (href.pathname ?? "#");
    return React.createElement("a", { href: resolved, ...rest }, children);
  }
}));

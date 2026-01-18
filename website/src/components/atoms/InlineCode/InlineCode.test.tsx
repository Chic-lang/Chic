import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { InlineCode } from "@/components/atoms/InlineCode/InlineCode";

describe("<InlineCode />", () => {
  it("renders children in a code element", () => {
    render(<InlineCode>hello</InlineCode>);
    expect(screen.getByText("hello").tagName).toBe("CODE");
  });
});


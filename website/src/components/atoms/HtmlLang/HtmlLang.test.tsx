import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { HtmlLang } from "@/components/atoms/HtmlLang/HtmlLang";

describe("<HtmlLang />", () => {
  it("sets documentElement.lang to the current locale", () => {
    document.documentElement.lang = "en-US";
    render(<HtmlLang locale="ja-JP" />);
    expect(document.documentElement.lang).toBe("ja-JP");
  });
});


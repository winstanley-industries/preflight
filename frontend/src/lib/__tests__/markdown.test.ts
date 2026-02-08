import { describe, it, expect } from "vitest";
import { renderMarkdown } from "../markdown";

describe("renderMarkdown", () => {
  it("renders bold text", () => {
    expect(renderMarkdown("**bold**")).toContain("<strong>bold</strong>");
  });

  it("renders italic text", () => {
    expect(renderMarkdown("*italic*")).toContain("<em>italic</em>");
  });

  it("renders inline code", () => {
    expect(renderMarkdown("`code`")).toContain("<code>code</code>");
  });

  it("renders fenced code blocks without syntax highlighting", () => {
    const result = renderMarkdown("```ts\nconst x = 1;\n```");
    expect(result).toContain("<code");
    expect(result).toContain("const x = 1;");
  });

  it("renders links with target=_blank and rel=noopener", () => {
    const result = renderMarkdown("[link](https://example.com)");
    expect(result).toContain('target="_blank"');
    expect(result).toContain('rel="noopener noreferrer"');
    expect(result).toContain("https://example.com");
  });

  it("renders unordered lists", () => {
    const result = renderMarkdown("- one\n- two");
    expect(result).toContain("<li>one</li>");
    expect(result).toContain("<li>two</li>");
  });

  it("renders blockquotes", () => {
    const result = renderMarkdown("> quote");
    expect(result).toContain("<blockquote>");
    expect(result).toContain("quote");
  });

  it("renders strikethrough (GFM)", () => {
    const result = renderMarkdown("~~deleted~~");
    expect(result).toContain("<del>deleted</del>");
  });

  it("renders tables (GFM)", () => {
    const result = renderMarkdown("| a | b |\n|---|---|\n| 1 | 2 |");
    expect(result).toContain("<table>");
    expect(result).toContain("<td>1</td>");
  });

  it("renders task lists (GFM)", () => {
    const result = renderMarkdown("- [x] done\n- [ ] todo");
    expect(result).toContain('type="checkbox"');
    expect(result).toContain("done");
    expect(result).toContain("todo");
  });

  it("strips script tags (XSS prevention)", () => {
    const result = renderMarkdown('<script>alert("xss")</script>');
    expect(result).not.toContain("<script>");
  });

  it("strips event handlers (XSS prevention)", () => {
    const result = renderMarkdown('<img onerror="alert(1)" src="x">');
    expect(result).not.toContain("onerror");
  });

  it("disables images (renders as text link)", () => {
    const result = renderMarkdown("![alt](https://example.com/img.png)");
    expect(result).not.toContain("<img");
  });

  it("returns empty string for empty input", () => {
    expect(renderMarkdown("")).toBe("");
  });

  it("handles plain text without markdown", () => {
    const result = renderMarkdown("just plain text");
    expect(result).toContain("just plain text");
  });
});

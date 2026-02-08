import DOMPurify from "dompurify";
import { Marked } from "marked";

const marked = new Marked({
  gfm: true,
  breaks: true,
});

function escapeAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

marked.use({
  renderer: {
    // Open links in new tab
    link({ href, title, tokens }) {
      const text = this.parser.parseInline(tokens);
      const safeHref = escapeAttr(href);
      const titleAttr = title ? ` title="${escapeAttr(title)}"` : "";
      return `<a href="${safeHref}"${titleAttr} target="_blank" rel="noopener noreferrer">${text}</a>`;
    },

    // Disable images — render as text links instead
    image({ href, title, text }) {
      const alt = text || title || href;
      return `<a href="${escapeAttr(href)}" target="_blank" rel="noopener noreferrer">${alt}</a>`;
    },
  },
});

export function renderMarkdown(body: string): string {
  if (!body) return "";
  const raw = marked.parse(body) as string;
  return DOMPurify.sanitize(raw, {
    ADD_ATTR: ["target"],
  });
}

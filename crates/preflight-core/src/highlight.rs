use syntect::html::ClassStyle;
use syntect::parsing::{ScopeStack, SyntaxSet};

/// Holds loaded syntaxes for reuse across requests.
pub struct Highlighter {
    syntax_set: SyntaxSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// Get the display name of a language by file extension.
    pub fn language_name(&self, ext: &str) -> Option<&str> {
        self.syntax_set
            .find_syntax_by_extension(ext)
            .map(|s| s.name.as_str())
    }

    /// Highlight a file's content, returning one HTML string per line.
    /// Returns `None` if the language is not recognized.
    /// Each line contains `<span class="sy-...">` elements with CSS classes.
    pub fn highlight_file(&self, content: &str, path: &str) -> Option<Vec<String>> {
        let ext = std::path::Path::new(path).extension()?.to_str()?;
        let syntax = self.syntax_set.find_syntax_by_extension(ext)?;

        let mut parse_state = syntect::parsing::ParseState::new(syntax);
        let mut scope_stack = ScopeStack::new();
        let mut lines = Vec::new();

        for line in syntect::util::LinesWithEndings::from(content) {
            let ops = parse_state.parse_line(line, &self.syntax_set).ok()?;
            let (html, _) = syntect::html::line_tokens_to_classed_spans(
                line,
                ops.as_slice(),
                ClassStyle::SpacedPrefixed { prefix: "sy-" },
                &mut scope_stack,
            )
            .ok()?;
            lines.push(html.trim_end_matches('\n').to_string());
        }

        Some(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighter() -> Highlighter {
        Highlighter::new()
    }

    #[test]
    fn language_name_works() {
        let hl = highlighter();
        assert_eq!(hl.language_name("rs"), Some("Rust"));
        assert_eq!(hl.language_name("js"), Some("JavaScript"));
        assert_eq!(hl.language_name("xyz123"), None);
    }

    #[test]
    fn highlights_rust_keyword() {
        let hl = highlighter();
        let lines = hl.highlight_file("fn main() {}\n", "test.rs").unwrap();
        assert_eq!(lines.len(), 1);
        // "fn" should be wrapped in a span with a sy- prefixed class
        assert!(lines[0].contains("sy-"), "expected sy- class prefix in: {}", lines[0]);
        assert!(lines[0].contains("fn"), "expected 'fn' keyword in: {}", lines[0]);
    }

    #[test]
    fn highlights_multiple_lines() {
        let hl = highlighter();
        let content = "let x = 1;\nlet y = 2;\n";
        let lines = hl.highlight_file(content, "test.rs").unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn returns_none_for_unknown_extension() {
        let hl = highlighter();
        let result = hl.highlight_file("hello world\n", "test.xyz123");
        assert!(result.is_none());
    }

    #[test]
    fn escapes_html_entities() {
        let hl = highlighter();
        let lines = hl.highlight_file("let x = a < b && c > d;\n", "test.rs").unwrap();
        assert_eq!(lines.len(), 1);
        // Should contain escaped entities, not raw < or >
        assert!(lines[0].contains("&lt;") || lines[0].contains("&amp;"),
            "expected HTML escaping in: {}", lines[0]);
    }

    #[test]
    fn handles_empty_content() {
        let hl = highlighter();
        let lines = hl.highlight_file("", "test.rs").unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn handles_javascript() {
        let hl = highlighter();
        let lines = hl.highlight_file("const x = 42;\n", "test.js").unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("sy-"));
    }

    #[test]
    fn no_trailing_newline_in_output() {
        let hl = highlighter();
        let lines = hl.highlight_file("fn main() {}\n", "test.rs").unwrap();
        // Individual line strings should not end with \n
        for line in &lines {
            assert!(!line.ends_with('\n'), "line should not end with newline: {line}");
        }
    }
}

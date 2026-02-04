use crate::diff::{DiffLine, FileDiff, FileStatus, Hunk, LineKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a unified diff string into a vector of `FileDiff` entries.
///
/// Splits on `diff --git` boundaries, extracts paths and file status,
/// then parses hunk headers and classifies individual diff lines.
pub fn parse_diff(input: &str) -> Result<Vec<FileDiff>, ParseError> {
    if input.is_empty() {
        return Ok(vec![]);
    }

    let lines: Vec<&str> = input.lines().collect();
    let mut file_diffs = Vec::new();

    // Find the start indices of each "diff --git" block
    let mut block_starts: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("diff --git ") {
            block_starts.push(i);
        }
    }

    if block_starts.is_empty() {
        return Ok(vec![]);
    }

    // Process each file block
    for (block_idx, &start) in block_starts.iter().enumerate() {
        let end = if block_idx + 1 < block_starts.len() {
            block_starts[block_idx + 1]
        } else {
            lines.len()
        };

        let file_diff = parse_file_block(&lines[start..end], start)?;
        file_diffs.push(file_diff);
    }

    Ok(file_diffs)
}

/// Parse a single file block (from one `diff --git` line to the next).
fn parse_file_block(block: &[&str], global_offset: usize) -> Result<FileDiff, ParseError> {
    let mut old_path: Option<String> = None;
    let mut new_path: Option<String> = None;
    let mut status = FileStatus::Modified;
    let mut hunks = Vec::new();
    let mut is_binary = false;

    let mut i = 1; // skip the "diff --git" line itself

    // Parse header lines until we hit a hunk or end of block
    while i < block.len() {
        let line = block[i];

        if line.starts_with("@@ ") {
            break;
        }

        if let Some(path) = line.strip_prefix("--- ") {
            old_path = if path == "/dev/null" {
                None
            } else {
                Some(strip_ab_prefix(path))
            };
        } else if let Some(path) = line.strip_prefix("+++ ") {
            new_path = if path == "/dev/null" {
                None
            } else {
                Some(strip_ab_prefix(path))
            };
        } else if line.starts_with("new file mode") {
            status = FileStatus::Added;
        } else if line.starts_with("deleted file mode") {
            status = FileStatus::Deleted;
        } else if let Some(from) = line.strip_prefix("rename from ") {
            status = FileStatus::Renamed;
            old_path = Some(from.to_string());
        } else if let Some(to) = line.strip_prefix("rename to ") {
            status = FileStatus::Renamed;
            new_path = Some(to.to_string());
        } else if line.starts_with("Binary files ") {
            is_binary = true;
        }

        i += 1;
    }

    if is_binary {
        status = FileStatus::Binary;
    }

    // Parse hunks
    while i < block.len() {
        if block[i].starts_with("@@ ") {
            let (hunk, next_i) = parse_hunk(block, i, global_offset)?;
            hunks.push(hunk);
            i = next_i;
        } else {
            i += 1;
        }
    }

    Ok(FileDiff {
        old_path,
        new_path,
        status,
        hunks,
    })
}

/// Strip the `a/` or `b/` prefix from a diff path.
fn strip_ab_prefix(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("a/").or_else(|| path.strip_prefix("b/")) {
        stripped.to_string()
    } else {
        path.to_string()
    }
}

/// Parse a single hunk starting at the `@@ ...` header line.
/// Returns the parsed `Hunk` and the index of the next line after the hunk.
fn parse_hunk(
    block: &[&str],
    start: usize,
    global_offset: usize,
) -> Result<(Hunk, usize), ParseError> {
    let header = block[start];
    let (old_start, old_count, new_start, new_count, context) =
        parse_hunk_header(header, global_offset + start)?;

    let mut diff_lines = Vec::new();
    let mut old_line = old_start;
    let mut new_line = new_start;
    let mut i = start + 1;

    while i < block.len() {
        let line = block[i];

        // Stop if we hit the next hunk header
        if line.starts_with("@@ ") {
            break;
        }

        // Skip "\ No newline at end of file"
        if line.starts_with('\\') {
            i += 1;
            continue;
        }

        if line.is_empty() {
            // Empty line in a diff is treated as a context line with empty content
            diff_lines.push(DiffLine {
                kind: LineKind::Context,
                content: String::new(),
                old_line_no: Some(old_line),
                new_line_no: Some(new_line),
            });
            old_line += 1;
            new_line += 1;
        } else {
            let prefix = &line[..1];
            let content = &line[1..];

            match prefix {
                " " => {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Context,
                        content: content.to_string(),
                        old_line_no: Some(old_line),
                        new_line_no: Some(new_line),
                    });
                    old_line += 1;
                    new_line += 1;
                }
                "+" => {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Added,
                        content: content.to_string(),
                        old_line_no: None,
                        new_line_no: Some(new_line),
                    });
                    new_line += 1;
                }
                "-" => {
                    diff_lines.push(DiffLine {
                        kind: LineKind::Removed,
                        content: content.to_string(),
                        old_line_no: Some(old_line),
                        new_line_no: None,
                    });
                    old_line += 1;
                }
                _ => {
                    // Unknown prefix; stop parsing this hunk
                    break;
                }
            }
        }

        i += 1;
    }

    Ok((
        Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            context,
            lines: diff_lines,
        },
        i,
    ))
}

/// Parse a hunk header like `@@ -10,6 +10,7 @@ fn existing_function() {`
fn parse_hunk_header(
    header: &str,
    line_no: usize,
) -> Result<(u32, u32, u32, u32, Option<String>), ParseError> {
    // Strip leading "@@ " and find closing " @@"
    let after_at = header.strip_prefix("@@ ").ok_or_else(|| ParseError {
        line: line_no + 1,
        message: "expected hunk header starting with '@@'".to_string(),
    })?;

    let closing_pos = after_at.find(" @@").ok_or_else(|| ParseError {
        line: line_no + 1,
        message: "expected closing '@@' in hunk header".to_string(),
    })?;

    let range_part = &after_at[..closing_pos];
    let after_closing = &after_at[closing_pos + 3..]; // skip " @@"

    let context = if after_closing.is_empty() {
        None
    } else {
        // Strip the leading space after @@
        let ctx = after_closing.strip_prefix(' ').unwrap_or(after_closing);
        if ctx.is_empty() {
            None
        } else {
            Some(ctx.to_string())
        }
    };

    // Parse "-old_start,old_count +new_start,new_count"
    let parts: Vec<&str> = range_part.split(' ').collect();
    if parts.len() != 2 {
        return Err(ParseError {
            line: line_no + 1,
            message: format!("expected two range specs, got {}", parts.len()),
        });
    }

    let (old_start, old_count) = parse_range(parts[0], '-', line_no)?;
    let (new_start, new_count) = parse_range(parts[1], '+', line_no)?;

    Ok((old_start, old_count, new_start, new_count, context))
}

/// Parse a range spec like `-10,6` or `+10,7` or `-1` (count omitted means 1).
fn parse_range(spec: &str, prefix: char, line_no: usize) -> Result<(u32, u32), ParseError> {
    let stripped = spec.strip_prefix(prefix).ok_or_else(|| ParseError {
        line: line_no + 1,
        message: format!("expected '{}' prefix in range spec '{}'", prefix, spec),
    })?;

    if let Some((start_str, count_str)) = stripped.split_once(',') {
        let start = start_str.parse::<u32>().map_err(|e| ParseError {
            line: line_no + 1,
            message: format!("invalid start in range '{}': {}", spec, e),
        })?;
        let count = count_str.parse::<u32>().map_err(|e| ParseError {
            line: line_no + 1,
            message: format!("invalid count in range '{}': {}", spec, e),
        })?;
        Ok((start, count))
    } else {
        let start = stripped.parse::<u32>().map_err(|e| ParseError {
            line: line_no + 1,
            message: format!("invalid start in range '{}': {}", spec, e),
        })?;
        Ok((start, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = parse_diff("");
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn test_single_modified_file_paths() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 use std::io;
+use std::fs;

 fn main() {
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path.as_deref(), Some("src/main.rs"));
        assert_eq!(result[0].new_path.as_deref(), Some("src/main.rs"));
        assert_eq!(result[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_new_file_status() {
        let input = "\
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,3 @@
+fn hello() {
+    println!(\"hello\");
+}
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, None);
        assert_eq!(result[0].new_path.as_deref(), Some("src/new.rs"));
        assert_eq!(result[0].status, FileStatus::Added);
    }

    #[test]
    fn test_deleted_file_status() {
        let input = "\
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
index abc1234..0000000
--- a/src/old.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-fn goodbye() {
-    println!(\"bye\");
-}
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path.as_deref(), Some("src/old.rs"));
        assert_eq!(result[0].new_path, None);
        assert_eq!(result[0].status, FileStatus::Deleted);
    }

    #[test]
    fn test_renamed_file_status() {
        let input = "\
diff --git a/src/old_name.rs b/src/new_name.rs
similarity index 95%
rename from src/old_name.rs
rename to src/new_name.rs
index abc1234..def5678 100644
--- a/src/old_name.rs
+++ b/src/new_name.rs
@@ -1,3 +1,3 @@
-fn old() {}
+fn new() {}
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path.as_deref(), Some("src/old_name.rs"));
        assert_eq!(result[0].new_path.as_deref(), Some("src/new_name.rs"));
        assert_eq!(result[0].status, FileStatus::Renamed);
    }

    #[test]
    fn test_binary_file() {
        let input = "\
diff --git a/image.png b/image.png
new file mode 100644
index 0000000..abc1234
Binary files /dev/null and b/image.png differ
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, FileStatus::Binary);
        assert!(result[0].hunks.is_empty());
    }

    #[test]
    fn test_multiple_files() {
        let input = "\
diff --git a/src/a.rs b/src/a.rs
index abc..def 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,3 @@
 line1
+line2
 line3
diff --git a/src/b.rs b/src/b.rs
index abc..def 100644
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,2 +1,2 @@
-old
+new
 same
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].new_path.as_deref(), Some("src/a.rs"));
        assert_eq!(result[1].new_path.as_deref(), Some("src/b.rs"));
    }

    #[test]
    fn test_hunk_header_parsing() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,6 +10,7 @@ fn existing_function() {
 context line
+added line
 more context
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result[0].hunks.len(), 1);
        let hunk = &result[0].hunks[0];
        assert_eq!(hunk.old_start, 10);
        assert_eq!(hunk.old_count, 6);
        assert_eq!(hunk.new_start, 10);
        assert_eq!(hunk.new_count, 7);
        assert_eq!(hunk.context.as_deref(), Some("fn existing_function() {"));
    }

    #[test]
    fn test_hunk_header_no_context() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 line1
+line2
 line3
 line4
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result[0].hunks[0].context, None);
    }

    #[test]
    fn test_line_classification() {
        use crate::diff::LineKind;
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,4 @@
 unchanged
-removed line
+added line
 also unchanged
";
        let result = parse_diff(input).unwrap();
        let lines = &result[0].hunks[0].lines;
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].kind, LineKind::Context);
        assert_eq!(lines[0].content, "unchanged");
        assert_eq!(lines[1].kind, LineKind::Removed);
        assert_eq!(lines[1].content, "removed line");
        assert_eq!(lines[2].kind, LineKind::Added);
        assert_eq!(lines[2].content, "added line");
        assert_eq!(lines[3].kind, LineKind::Context);
        assert_eq!(lines[3].content, "also unchanged");
    }

    #[test]
    fn test_line_numbers() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -5,4 +5,5 @@
 context
-old
+new1
+new2
 context
";
        let result = parse_diff(input).unwrap();
        let lines = &result[0].hunks[0].lines;
        assert_eq!(lines[0].old_line_no, Some(5));
        assert_eq!(lines[0].new_line_no, Some(5));
        assert_eq!(lines[1].old_line_no, Some(6));
        assert_eq!(lines[1].new_line_no, None);
        assert_eq!(lines[2].old_line_no, None);
        assert_eq!(lines[2].new_line_no, Some(6));
        assert_eq!(lines[3].old_line_no, None);
        assert_eq!(lines[3].new_line_no, Some(7));
        assert_eq!(lines[4].old_line_no, Some(7));
        assert_eq!(lines[4].new_line_no, Some(8));
    }

    #[test]
    fn test_multiple_hunks() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 first
+inserted
 second
 third
@@ -10,3 +11,4 @@
 tenth
+another
 eleventh
 twelfth
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result[0].hunks.len(), 2);
        assert_eq!(result[0].hunks[0].old_start, 1);
        assert_eq!(result[0].hunks[1].old_start, 10);
        assert_eq!(result[0].hunks[1].new_start, 11);
    }

    #[test]
    fn test_empty_line_in_hunk() {
        let input = "diff --git a/f b/f\nindex abc..def 100644\n--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n first\n\n last\n";
        let result = parse_diff(input).unwrap();
        let lines = &result[0].hunks[0].lines;
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].kind, LineKind::Context);
        assert_eq!(lines[0].content, "first");
        assert_eq!(lines[1].kind, LineKind::Context);
        assert_eq!(lines[1].content, "");
        assert_eq!(lines[2].kind, LineKind::Context);
        assert_eq!(lines[2].content, "last");
    }

    #[test]
    fn test_hunk_count_omitted_means_one() {
        let input = "\
diff --git a/f b/f
index abc..def 100644
--- a/f
+++ b/f
@@ -1 +1,2 @@
 only line
+new line
";
        let result = parse_diff(input).unwrap();
        let hunk = &result[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 2);
    }
}

use crate::diff::{DiffLine, Hunk, LineKind};

/// Reconstruct full file content by applying hunks to base content.
/// Takes the "old" base content and hunks, returns the "new" content after applying the hunks.
pub fn reconstruct_from_hunks(base_content: &str, hunks: &[Hunk]) -> String {
    // Build a map of which base lines are affected by hunks
    // For each hunk, we know old_start and old_count
    // Lines not covered by any hunk come from base unchanged
    // Lines within a hunk: use the "new side" (Added + Context lines)

    let base_lines: Vec<&str> = base_content.lines().collect();
    let mut result = String::new();
    let mut base_idx = 0; // 0-indexed position in base_lines

    for hunk in hunks {
        let hunk_start = if hunk.old_start > 0 {
            (hunk.old_start - 1) as usize
        } else {
            0
        };

        // Copy base lines before this hunk
        while base_idx < hunk_start && base_idx < base_lines.len() {
            result.push_str(base_lines[base_idx]);
            result.push('\n');
            base_idx += 1;
        }

        // Apply hunk: output the new side
        for line in &hunk.lines {
            match line.kind {
                LineKind::Removed => {
                    // Skip removed lines
                }
                // Context, Added, and any future variants: include in output
                _ => {
                    result.push_str(&line.content);
                    result.push('\n');
                }
            }
        }

        // Advance base_idx past the old lines covered by this hunk
        base_idx = hunk_start + hunk.old_count as usize;
    }

    // Copy remaining base lines
    while base_idx < base_lines.len() {
        result.push_str(base_lines[base_idx]);
        result.push('\n');
        base_idx += 1;
    }

    result
}

/// Compute the interdiff between two revisions of the same file.
/// Takes the original base content and hunks from each revision.
/// Returns a unified diff between the "from" version and the "to" version.
pub fn compute_interdiff(base_content: &str, from_hunks: &[Hunk], to_hunks: &[Hunk]) -> Vec<Hunk> {
    let from_content = reconstruct_from_hunks(base_content, from_hunks);
    let to_content = reconstruct_from_hunks(base_content, to_hunks);

    // Use `similar` crate to compute a unified diff
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(&from_content, &to_content);
    let mut hunks = Vec::new();

    for group in diff.grouped_ops(3) {
        let mut lines = Vec::new();
        let mut old_start = 0u32;
        let mut old_count = 0u32;
        let mut new_start = 0u32;
        let mut new_count = 0u32;
        let mut first = true;

        for op in &group {
            for change in diff.iter_changes(op) {
                let content = change.value().trim_end_matches('\n').to_string();
                if first {
                    old_start = (change.old_index().unwrap_or(0) + 1) as u32;
                    new_start = (change.new_index().unwrap_or(0) + 1) as u32;
                    first = false;
                }
                match change.tag() {
                    ChangeTag::Equal => {
                        old_count += 1;
                        new_count += 1;
                        lines.push(DiffLine {
                            kind: LineKind::Context,
                            content,
                            old_line_no: Some(old_start + old_count - 1),
                            new_line_no: Some(new_start + new_count - 1),
                            highlighted: None,
                        });
                    }
                    ChangeTag::Delete => {
                        old_count += 1;
                        lines.push(DiffLine {
                            kind: LineKind::Removed,
                            content,
                            old_line_no: Some(old_start + old_count - 1),
                            new_line_no: None,
                            highlighted: None,
                        });
                    }
                    ChangeTag::Insert => {
                        new_count += 1;
                        lines.push(DiffLine {
                            kind: LineKind::Added,
                            content,
                            old_line_no: None,
                            new_line_no: Some(new_start + new_count - 1),
                            highlighted: None,
                        });
                    }
                }
            }
        }

        if !lines.is_empty() {
            hunks.push(Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
                context: None,
                lines,
            });
        }
    }

    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hunk(
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
        lines: Vec<DiffLine>,
    ) -> Hunk {
        Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            context: None,
            lines,
        }
    }

    fn ctx(content: &str, old: u32, new: u32) -> DiffLine {
        DiffLine {
            kind: LineKind::Context,
            content: content.into(),
            old_line_no: Some(old),
            new_line_no: Some(new),
            highlighted: None,
        }
    }

    fn add(content: &str, new: u32) -> DiffLine {
        DiffLine {
            kind: LineKind::Added,
            content: content.into(),
            old_line_no: None,
            new_line_no: Some(new),
            highlighted: None,
        }
    }

    fn rem(content: &str, old: u32) -> DiffLine {
        DiffLine {
            kind: LineKind::Removed,
            content: content.into(),
            old_line_no: Some(old),
            new_line_no: None,
            highlighted: None,
        }
    }

    #[test]
    fn reconstruct_simple_addition() {
        let base = "line1\nline2\nline3\n";
        let hunks = vec![make_hunk(
            1,
            3,
            1,
            4,
            vec![
                ctx("line1", 1, 1),
                add("new_line", 2),
                ctx("line2", 2, 3),
                ctx("line3", 3, 4),
            ],
        )];
        let result = reconstruct_from_hunks(base, &hunks);
        assert_eq!(result, "line1\nnew_line\nline2\nline3\n");
    }

    #[test]
    fn no_changes_between_revisions() {
        let base = "a\nb\nc\n";
        let hunks = vec![make_hunk(
            1,
            3,
            1,
            4,
            vec![ctx("a", 1, 1), add("x", 2), ctx("b", 2, 3), ctx("c", 3, 4)],
        )];
        let result = compute_interdiff(base, &hunks, &hunks);
        assert!(
            result.is_empty()
                || result
                    .iter()
                    .all(|h| h.lines.iter().all(|l| l.kind == LineKind::Context))
        );
    }

    #[test]
    fn interdiff_detects_added_line() {
        let base = "a\nb\nc\n";
        let from_hunks = vec![make_hunk(
            1,
            3,
            1,
            4,
            vec![ctx("a", 1, 1), add("x", 2), ctx("b", 2, 3), ctx("c", 3, 4)],
        )];
        let to_hunks = vec![make_hunk(
            1,
            3,
            1,
            5,
            vec![
                ctx("a", 1, 1),
                add("x", 2),
                add("y", 3),
                ctx("b", 2, 4),
                ctx("c", 3, 5),
            ],
        )];
        let interdiff = compute_interdiff(base, &from_hunks, &to_hunks);
        let added: Vec<_> = interdiff
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Added)
            .collect();
        assert!(!added.is_empty());
        assert_eq!(added[0].content, "y");
    }

    #[allow(dead_code)]
    fn _use_helpers() {
        // Ensure rem helper is used to avoid dead_code warning
        let _ = rem("x", 1);
    }
}

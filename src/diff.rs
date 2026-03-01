//! Diff generation and colored rendering for file changes.
//!
//! Provides [`unified_diff`] for comparing old vs new content and
//! [`new_file_preview`] for all-additions preview of new files.
//! Used by [`crate::hooks::KazeHook`] for pre-write diff display.

use colored::Colorize;
use similar::{ChangeTag, TextDiff};

/// Generate a colored unified diff string.
///
/// Compares `old` and `new` content line-by-line and produces a unified diff
/// with colored additions (green) and deletions (red). Returns an empty string
/// if the contents are identical.
pub fn unified_diff(old: &str, new: &str, path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    // Header
    output.push_str(&format!("--- a/{}\n", path));
    output.push_str(&format!("+++ b/{}\n", path));

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{}", hunk.header()));

        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    output.push_str(&format!("{}", format!("-{}", change).red()));
                }
                ChangeTag::Insert => {
                    output.push_str(&format!("{}", format!("+{}", change).green()));
                }
                ChangeTag::Equal => {
                    output.push_str(&format!(" {}", change));
                }
            };
        }
    }

    output
}

/// Generate a colored preview for a new file (all lines are additions).
///
/// Used when write_file targets a path that doesn't exist yet.
pub fn new_file_preview(content: &str, path: &str) -> String {
    let mut output = String::new();
    output.push_str("--- /dev/null\n");
    output.push_str(&format!("+++ b/{}\n", path));

    for line in content.lines() {
        output.push_str(&format!("{}", format!("+{}\n", line).green()));
    }

    output
}

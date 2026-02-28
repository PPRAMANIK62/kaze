use colored::Colorize;

use crate::message::{Message, Role};

/// Format a message for terminal display with role label and colors.
pub fn format_message(msg: &Message) -> String {
    let label = format_role_label(&msg.role);
    let body = format_body(msg.text(), &msg.role);
    format!("{}\n{}", label, body)
}

fn format_role_label(role: &Role) -> String {
    match role {
        Role::User => format!("{}", "you:".green().bold()),
        Role::Assistant => format!("{}", "kaze:".cyan().bold()),
        Role::System => format!("{}", "system:".dimmed()),
        Role::Tool => format!("{}", "tool:".yellow()),
    }
}

/// Apply basic markdown-lite formatting to text.
/// Handles: **bold**, `inline code`, and ```code blocks```.
fn format_body(text: &str, role: &Role) -> String {
    match role {
        Role::User => text.to_string(),
        Role::Assistant => render_markdown_lite(text),
        _ => text.dimmed().to_string(),
    }
}

/// Minimal markdown renderer for terminal output.
/// Not a full parser. Handles the three most common patterns
/// in LLM output: bold, inline code, and fenced code blocks.
pub fn render_markdown_lite(text: &str) -> String {
    let mut output = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                code_lang.clear();
                output.push('\n');
            } else {
                in_code_block = true;
                code_lang = line.trim_start_matches('`').to_string();
                if !code_lang.is_empty() {
                    output.push_str(&format!("  {}\n", code_lang.dimmed()));
                }
            }
            continue;
        }

        if in_code_block {
            output.push_str(&format!("  {}\n", line.dimmed()));
            continue;
        }

        let formatted = render_inline(line);
        output.push_str(&formatted);
        output.push('\n');
    }

    if output.ends_with('\n') {
        output.pop();
    }
    output
}

/// Handle **bold** and `inline code` within a single line.
fn render_inline(line: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing(&chars, i + 2, "**") {
                let bold_text: String = chars[i + 2..end].iter().collect();
                result.push_str(&bold_text.bold().to_string());
                i = end + 2;
                continue;
            }
        }

        if chars[i] == '`' {
            if let Some(end) = find_closing_char(&chars, i + 1, '`') {
                let code_text: String = chars[i + 1..end].iter().collect();
                result.push_str(&code_text.dimmed().to_string());
                i = end + 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

fn find_closing(chars: &[char], start: usize, pattern: &str) -> Option<usize> {
    let pat: Vec<char> = pattern.chars().collect();
    for i in start..chars.len() - pat.len() + 1 {
        if chars[i..i + pat.len()] == pat[..] {
            return Some(i);
        }
    }
    None
}

fn find_closing_char(chars: &[char], start: usize, ch: char) -> Option<usize> {
    for i in start..chars.len() {
        if chars[i] == ch {
            return Some(i);
        }
    }
    None
}

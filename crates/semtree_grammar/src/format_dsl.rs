//! Pretty-printer for `.semtree` grammar files.
//!
//! Formats the DSL while preserving comments, section headers, and rule order.
//! Does not round-trip through Grammar IR (that would drop comments).

/// Format a SemTree DSL source string.
pub fn format_semtree_dsl(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut last_was_blank = true;
    let mut in_keyword_block = false;

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();

        // Blank line
        if trimmed.is_empty() {
            // Collapse multiple blanks to one, but keep section spacing.
            if !last_was_blank && !out.is_empty() {
                out.push('\n');
                last_was_blank = true;
            }
            in_keyword_block = false;
            i += 1;
            continue;
        }

        // Comment / section header
        if trimmed.starts_with('#') {
            if in_keyword_block && !last_was_blank {
                out.push('\n');
            }
            out.push_str(trimmed);
            out.push('\n');
            last_was_blank = false;
            in_keyword_block = false;
            i += 1;
            continue;
        }

        // Directives
        if let Some(rest) = trimmed.strip_prefix("language ") {
            ensure_blank_before(&mut out, &mut last_was_blank, false);
            out.push_str("language ");
            out.push_str(rest.trim());
            out.push('\n');
            last_was_blank = false;
            in_keyword_block = false;
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("keyword ") {
            if !in_keyword_block && !last_was_blank {
                out.push('\n');
            }
            out.push_str("keyword ");
            out.push_str(rest.trim());
            out.push('\n');
            last_was_blank = false;
            in_keyword_block = true;
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("extra ") {
            if in_keyword_block && !last_was_blank {
                out.push('\n');
            }
            out.push_str("extra ");
            out.push_str(rest.trim());
            out.push('\n');
            last_was_blank = false;
            in_keyword_block = false;
            i += 1;
            continue;
        }

        if trimmed.starts_with("indent ")
            || trimmed.starts_with("linebreak ")
            || trimmed.starts_with("space ")
        {
            if in_keyword_block && !last_was_blank {
                out.push('\n');
            }
            out.push_str(trimmed);
            out.push('\n');
            last_was_blank = false;
            in_keyword_block = false;
            i += 1;
            continue;
        }

        // Rule definition: Name := ...
        if trimmed.contains(":=") {
            if !last_was_blank {
                out.push('\n');
            }
            i = format_rule(&lines, i, &mut out);
            last_was_blank = false;
            in_keyword_block = false;
            continue;
        }

        // Unknown line — keep as-is (trimmed)
        out.push_str(trimmed);
        out.push('\n');
        last_was_blank = false;
        in_keyword_block = false;
        i += 1;
    }

    // Ensure trailing newline
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

fn ensure_blank_before(out: &mut String, last_was_blank: &mut bool, force: bool) {
    if force && !*last_was_blank && !out.is_empty() {
        out.push('\n');
        *last_was_blank = true;
    }
}

/// Format a rule starting at `i`. Returns the next index after the rule.
fn format_rule(lines: &[&str], start: usize, out: &mut String) -> usize {
    let trimmed = lines[start].trim();
    let parts: Vec<&str> = trimmed.splitn(2, ":=").collect();
    let name = parts[0].trim();
    let same_line_body = parts.get(1).map(|s| s.trim()).unwrap_or("");

    out.push_str(name);
    out.push_str(" :=");

    let mut i = start + 1;
    let mut body_parts: Vec<String> = Vec::new();

    if !same_line_body.is_empty() {
        body_parts.push(same_line_body.to_string());
    }

    // Collect indented body lines
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();

        if t.is_empty() {
            // Blank ends the rule body
            break;
        }
        if t.starts_with('#') {
            // Comment at column 0 ends the rule; indented comments stay in body
            if !line.starts_with(' ') && !line.starts_with('\t') {
                break;
            }
            body_parts.push(t.to_string());
            i += 1;
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }

        body_parts.push(t.to_string());
        i += 1;
    }

    if body_parts.is_empty() {
        out.push('\n');
        return i;
    }

    // Join body, then pretty-print choices
    let joined = body_parts.join(" ");
    let alternatives = split_top_level_choices(&joined);

    if alternatives.len() == 1 {
        let alt = alternatives[0].trim();
        // Short single-line body stays on one indented line
        if alt.len() <= 72 && !alt.contains('|') {
            out.push('\n');
            out.push_str("    ");
            out.push_str(alt);
            out.push('\n');
        } else {
            out.push('\n');
            out.push_str("    ");
            out.push_str(alt);
            out.push('\n');
        }
    } else {
        // Multi-alternative: first on its own line, rest as "| ..."
        out.push('\n');
        for (idx, alt) in alternatives.iter().enumerate() {
            let a = alt.trim();
            if a.is_empty() {
                continue;
            }
            if idx == 0 {
                out.push_str("    ");
                out.push_str(a);
            } else {
                out.push_str("    | ");
                out.push_str(a);
            }
            out.push('\n');
        }
    }

    i
}

/// Split on `|` that are not inside quotes or parentheses.
fn split_top_level_choices(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_string => {
                in_string = true;
                current.push(c);
            }
            '"' if in_string => {
                // Handle escaped quotes
                current.push(c);
                in_string = false;
            }
            '\\' if in_string => {
                current.push(c);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '(' | '[' | '{' if !in_string => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' if !in_string => {
                depth -= 1;
                current.push(c);
            }
            '|' if !in_string && depth == 0 => {
                result.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() || result.is_empty() {
        result.push(current);
    }

    // If the original had leading "| " style continuations already split
    // into body_parts, we may get empty first parts — filter them.
    result
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_simple_grammar() {
        let src = r#"
language json
keyword true
keyword false

Document:=Value
Value:=Object|Array|String
Object:="{" PairList? "}"
"#;
        let formatted = format_semtree_dsl(src);
        assert!(formatted.contains("language json\n"));
        assert!(formatted.contains("keyword true\n"));
        assert!(formatted.contains("Document :="));
        assert!(formatted.contains("    Value\n"));
        assert!(formatted.contains("    Object"));
        assert!(formatted.contains("    | Array"));
        assert!(formatted.contains("    | String"));
    }

    #[test]
    fn preserves_comments() {
        let src = "# header\nlanguage x\n\n# section\nRule :=\n    Foo\n";
        let formatted = format_semtree_dsl(src);
        assert!(formatted.contains("# header\n"));
        assert!(formatted.contains("# section\n"));
    }

    #[test]
    fn formats_keyword_block() {
        let src = "language py\nkeyword def\nkeyword class\n\nModule :=\n    Statement*\n";
        let formatted = format_semtree_dsl(src);
        assert!(formatted.starts_with("language py\n\nkeyword def\nkeyword class\n"));
    }

    #[test]
    fn does_not_split_pipe_in_strings() {
        let src = r#"
language x
Rule :=
    "a|b" | Other
"#;
        let formatted = format_semtree_dsl(src);
        assert!(formatted.contains(r#""a|b""#));
        assert!(formatted.contains("| Other"));
    }
}

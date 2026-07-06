use text_size::TextRange;

use crate::rename::TextEdit;

/// The result of an extract-variable refactoring.
#[derive(Debug, Clone)]
pub struct Extraction {
    pub edits: Vec<TextEdit>,
    pub new_name: String,
}

/// Extract the expression in the given selection into a new variable.
pub fn extract_variable(source: &str, selection: TextRange) -> Option<Extraction> {
    let start = u32::from(selection.start()) as usize;
    let end = u32::from(selection.end()) as usize;

    if start >= end || end > source.len() {
        return None;
    }

    let selected_text = &source[start..end];
    if selected_text.trim().is_empty() {
        return None;
    }

    let new_name = "extracted".to_string();

    // Find the start of the line containing the selection to insert the let binding.
    let insert_offset = source[..start]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);

    let indent = &source[insert_offset..start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();

    let binding = format!("{}let {} = {};\n", indent, new_name, selected_text.trim());

    let mut edits = Vec::new();

    // Insert the let binding before the current line
    edits.push(TextEdit {
        range: TextRange::new(
            (insert_offset as u32).into(),
            (insert_offset as u32).into(),
        ),
        new_text: binding,
    });

    // Replace the selection with the variable name
    edits.push(TextEdit {
        range: selection,
        new_text: new_name.clone(),
    });

    Some(Extraction { edits, new_name })
}

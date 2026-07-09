use text_size::TextRange;

/// A programmatic tree editor that accumulates mutations and applies them to source text.
#[derive(Debug, Clone)]
pub struct TreeEditor {
    source: String,
    edits: Vec<EditOp>,
}

#[derive(Debug, Clone)]
enum EditOp {
    Replace { range: TextRange, new_text: String },
    InsertBefore { offset: u32, text: String },
    InsertAfter { offset: u32, text: String },
    Remove { range: TextRange },
}

impl TreeEditor {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            edits: Vec::new(),
        }
    }

    /// Replace a node's text range with new text.
    pub fn replace_node(&mut self, range: TextRange, new_text: &str) {
        self.edits.push(EditOp::Replace {
            range,
            new_text: new_text.to_string(),
        });
    }

    /// Insert text before the given offset.
    pub fn insert_before(&mut self, offset: u32, text: &str) {
        self.edits.push(EditOp::InsertBefore {
            offset,
            text: text.to_string(),
        });
    }

    /// Insert text after the given offset.
    pub fn insert_after(&mut self, offset: u32, text: &str) {
        self.edits.push(EditOp::InsertAfter {
            offset,
            text: text.to_string(),
        });
    }

    /// Remove a node by its text range.
    pub fn remove_node(&mut self, range: TextRange) {
        self.edits.push(EditOp::Remove { range });
    }

    /// Apply all accumulated edits and return the resulting source.
    pub fn apply(self) -> String {
        // Convert all edits into (offset, delete_len, insert_text) and sort by offset descending
        let mut ops: Vec<(u32, u32, String)> = self
            .edits
            .iter()
            .map(|e| match e {
                EditOp::Replace { range, new_text } => (
                    u32::from(range.start()),
                    u32::from(range.len()),
                    new_text.clone(),
                ),
                EditOp::InsertBefore { offset, text } => (*offset, 0, text.clone()),
                EditOp::InsertAfter { offset, text } => (*offset, 0, text.clone()),
                EditOp::Remove { range } => (
                    u32::from(range.start()),
                    u32::from(range.len()),
                    String::new(),
                ),
            })
            .collect();

        // Sort by offset descending so earlier edits don't shift later offsets
        ops.sort_by_key(|a| std::cmp::Reverse(a.0));

        let mut result = self.source;
        for (offset, delete_len, insert) in ops {
            let start = offset as usize;
            let end = start + delete_len as usize;
            if end <= result.len() {
                result.replace_range(start..end, &insert);
            }
        }
        result
    }
}

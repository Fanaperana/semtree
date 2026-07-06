use text_size::{TextRange, TextSize};

/// A span in the source text, combining a range with an optional file id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub range: TextRange,
    pub file_id: u32,
}

impl TextSpan {
    pub fn new(file_id: u32, range: TextRange) -> Self {
        Self { range, file_id }
    }

    pub fn from_offsets(file_id: u32, start: u32, end: u32) -> Self {
        Self {
            range: TextRange::new(TextSize::new(start), TextSize::new(end)),
            file_id,
        }
    }

    pub fn start(&self) -> TextSize {
        self.range.start()
    }

    pub fn end(&self) -> TextSize {
        self.range.end()
    }

    pub fn len(&self) -> TextSize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    pub fn contains(&self, offset: TextSize) -> bool {
        self.range.contains(offset)
    }
}

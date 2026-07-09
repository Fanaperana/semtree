/// Configuration for the formatter.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub indent_size: usize,
    pub use_tabs: bool,
    pub max_line_width: usize,
    pub trailing_newline: bool,
    pub space_before_brace: bool,
    pub space_around_operators: bool,
    pub space_after_comma: bool,
    pub space_after_colon: bool,
    pub blank_line_between_items: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            use_tabs: false,
            max_line_width: 100,
            trailing_newline: true,
            space_before_brace: true,
            space_around_operators: true,
            space_after_comma: true,
            space_after_colon: true,
            blank_line_between_items: true,
        }
    }
}

impl FormatConfig {
    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        }
    }
}

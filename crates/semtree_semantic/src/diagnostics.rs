use smol_str::SmolStr;
use text_size::TextRange;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    pub severity: DiagnosticSeverity,
    pub code: Option<SmolStr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Info => write!(f, "info"),
            DiagnosticSeverity::Hint => write!(f, "hint"),
        }
    }
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, range: TextRange) -> Self {
        Self {
            message: message.into(),
            range,
            severity: DiagnosticSeverity::Error,
            code: None,
        }
    }

    pub fn warning(message: impl Into<String>, range: TextRange) -> Self {
        Self {
            message: message.into(),
            range,
            severity: DiagnosticSeverity::Warning,
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<SmolStr>) -> Self {
        self.code = Some(code.into());
        self
    }
}

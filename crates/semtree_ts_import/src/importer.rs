use semtree_grammar::ir::{Grammar, Rule, RuleExpr};
use serde_json::Value;

/// Errors that can occur during Tree-sitter grammar import.
#[derive(Debug)]
pub enum TsImportError {
    JsonParse(serde_json::Error),
    MissingField(String),
    UnsupportedConstruct(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for TsImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TsImportError::JsonParse(e) => write!(f, "JSON parse error: {e}"),
            TsImportError::MissingField(s) => write!(f, "missing field: {s}"),
            TsImportError::UnsupportedConstruct(s) => write!(f, "unsupported: {s}"),
            TsImportError::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for TsImportError {}

impl From<serde_json::Error> for TsImportError {
    fn from(e: serde_json::Error) -> Self {
        TsImportError::JsonParse(e)
    }
}

impl From<std::io::Error> for TsImportError {
    fn from(e: std::io::Error) -> Self {
        TsImportError::IoError(e)
    }
}

/// Import a Tree-sitter `grammar.json` (the compiled form of grammar.js)
/// and convert it to SemTree's Grammar IR.
///
/// Tree-sitter grammars compile `grammar.js` -> `grammar.json` via
/// `tree-sitter generate`. This function reads that JSON.
pub fn import_tree_sitter_grammar(json_str: &str) -> Result<Grammar, TsImportError> {
    let value: Value = serde_json::from_str(json_str)?;

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TsImportError::MissingField("name".into()))?;

    let mut grammar = Grammar::new(name);

    // Import extras (whitespace/comment patterns)
    if let Some(extras) = value.get("extras").and_then(|v| v.as_array()) {
        for extra in extras {
            if let Some(pattern) = extra.get("value").and_then(|v| v.as_str()) {
                grammar.extras.push(pattern.into());
            }
        }
    }

    // Import rules
    if let Some(rules) = value.get("rules").and_then(|v| v.as_object()) {
        for (rule_name, rule_value) in rules {
            let expr = convert_rule_expr(rule_value)?;
            let rule = Rule {
                name: rule_name.as_str().into(),
                expr,
                fields: Vec::new(), // fields extracted separately
            };
            grammar.add_rule(rule_name.as_str(), rule);
        }
    }

    // Import word (keyword identifier) token
    if let Some(word) = value.get("word").and_then(|v| v.as_str()) {
        grammar.add_keyword(word);
    }

    Ok(grammar)
}

fn convert_rule_expr(value: &Value) -> Result<RuleExpr, TsImportError> {
    let rule_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("BLANK");

    match rule_type {
        "BLANK" => Ok(RuleExpr::Blank),

        "STRING" => {
            let s = value.get("value").and_then(|v| v.as_str()).unwrap_or("");
            Ok(RuleExpr::Literal(s.into()))
        }

        "PATTERN" => {
            let pattern = value.get("value").and_then(|v| v.as_str()).unwrap_or(".*");
            Ok(RuleExpr::Token(Box::new(RuleExpr::Literal(pattern.into()))))
        }

        "SYMBOL" => {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| TsImportError::MissingField("symbol name".into()))?;
            Ok(RuleExpr::RuleRef(name.into()))
        }

        "SEQ" => {
            let members = value
                .get("members")
                .and_then(|v| v.as_array())
                .ok_or_else(|| TsImportError::MissingField("seq members".into()))?;
            let exprs: Result<Vec<_>, _> = members.iter().map(convert_rule_expr).collect();
            Ok(RuleExpr::Seq(exprs?))
        }

        "CHOICE" => {
            let members = value
                .get("members")
                .and_then(|v| v.as_array())
                .ok_or_else(|| TsImportError::MissingField("choice members".into()))?;
            let exprs: Result<Vec<_>, _> = members.iter().map(convert_rule_expr).collect();
            Ok(RuleExpr::Choice(exprs?))
        }

        "REPEAT" => {
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("repeat content".into()))?;
            Ok(RuleExpr::Repeat(Box::new(convert_rule_expr(content)?)))
        }

        "REPEAT1" => {
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("repeat1 content".into()))?;
            Ok(RuleExpr::Repeat1(Box::new(convert_rule_expr(content)?)))
        }

        "ALIAS" => {
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("alias content".into()))?;
            convert_rule_expr(content)
        }

        "FIELD" => {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| TsImportError::MissingField("field name".into()))?;
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("field content".into()))?;
            Ok(RuleExpr::Field(
                name.into(),
                Box::new(convert_rule_expr(content)?),
            ))
        }

        "TOKEN" | "IMMEDIATE_TOKEN" => {
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("token content".into()))?;
            Ok(RuleExpr::Token(Box::new(convert_rule_expr(content)?)))
        }

        "PREC" | "PREC_DYNAMIC" => {
            let prec = value.get("value").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("prec content".into()))?;
            Ok(RuleExpr::Prec(prec, Box::new(convert_rule_expr(content)?)))
        }

        "PREC_LEFT" => {
            let prec = value.get("value").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("prec_left content".into()))?;
            Ok(RuleExpr::PrecLeft(
                prec,
                Box::new(convert_rule_expr(content)?),
            ))
        }

        "PREC_RIGHT" => {
            let prec = value.get("value").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let content = value
                .get("content")
                .ok_or_else(|| TsImportError::MissingField("prec_right content".into()))?;
            Ok(RuleExpr::PrecRight(
                prec,
                Box::new(convert_rule_expr(content)?),
            ))
        }

        other => Ok(RuleExpr::Literal(
            format!("TODO_UNSUPPORTED_{other}").into(),
        )),
    }
}

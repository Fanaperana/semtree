use crate::import_tree_sitter_grammar;

#[test]
fn import_minimal_grammar() {
    let json = r#"{
        "name": "test_lang",
        "word": "identifier",
        "rules": {
            "source_file": {
                "type": "REPEAT",
                "content": {
                    "type": "SYMBOL",
                    "name": "expression"
                }
            },
            "expression": {
                "type": "CHOICE",
                "members": [
                    {
                        "type": "SYMBOL",
                        "name": "identifier"
                    },
                    {
                        "type": "SYMBOL",
                        "name": "number"
                    }
                ]
            },
            "identifier": {
                "type": "PATTERN",
                "value": "[a-zA-Z_][a-zA-Z0-9_]*"
            },
            "number": {
                "type": "PATTERN",
                "value": "[0-9]+"
            }
        },
        "extras": [
            { "type": "PATTERN", "value": "\\s" }
        ]
    }"#;

    let grammar = import_tree_sitter_grammar(json).unwrap();
    assert_eq!(grammar.name.as_str(), "test_lang");
    assert!(grammar.rules.contains_key("source_file"));
    assert!(grammar.rules.contains_key("expression"));
    assert!(grammar.rules.contains_key("identifier"));
}

#[test]
fn import_seq_and_field() {
    let json = r#"{
        "name": "mini",
        "rules": {
            "function": {
                "type": "SEQ",
                "members": [
                    { "type": "STRING", "value": "fn" },
                    {
                        "type": "FIELD",
                        "name": "name",
                        "content": { "type": "SYMBOL", "name": "identifier" }
                    }
                ]
            },
            "identifier": {
                "type": "PATTERN",
                "value": "[a-z]+"
            }
        }
    }"#;

    let grammar = import_tree_sitter_grammar(json).unwrap();
    assert!(grammar.rules.contains_key("function"));
}

#[test]
fn import_prec() {
    let json = r#"{
        "name": "prec_test",
        "rules": {
            "binary_expr": {
                "type": "PREC_LEFT",
                "value": 1,
                "content": {
                    "type": "SEQ",
                    "members": [
                        { "type": "SYMBOL", "name": "expr" },
                        { "type": "STRING", "value": "+" },
                        { "type": "SYMBOL", "name": "expr" }
                    ]
                }
            },
            "expr": {
                "type": "PATTERN",
                "value": "[0-9]+"
            }
        }
    }"#;

    let grammar = import_tree_sitter_grammar(json).unwrap();
    assert!(grammar.rules.contains_key("binary_expr"));
}

use std::path::PathBuf;

pub fn init(name: Option<String>, output: PathBuf) -> super::Result {
    let lang_name = name.unwrap_or_else(|| "my_language".to_string());
    let dir = output.join(&lang_name);

    std::fs::create_dir_all(&dir)?;

    let grammar_content = format!(
        r#"# {lang_name} grammar definition
language {lang_name}

# Keywords
keyword if
keyword else
keyword fn
keyword let
keyword return

# Rules
SourceFile :=
    Statement*

Statement :=
    LetStatement | ExpressionStatement

LetStatement :=
    "let" name: Identifier "=" Expression ";"

ExpressionStatement :=
    Expression ";"

Expression :=
    Identifier | Literal

Literal :=
    Integer | String

indent Block
linebreak Function
space around "+"
"#
    );

    std::fs::write(dir.join("grammar.semtree"), grammar_content)?;

    let config = serde_json::json!({
        "name": lang_name,
        "version": "0.1.0",
        "grammar": "grammar.semtree",
        "file_extensions": [format!(".{lang_name}")],
    });
    std::fs::write(
        dir.join("semtree.json"),
        serde_json::to_string_pretty(&config)?,
    )?;

    println!("Initialized SemTree project: {lang_name}");
    println!("  {}", dir.display());
    println!("  grammar.semtree");
    println!("  semtree.json");

    Ok(())
}

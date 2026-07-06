use std::path::PathBuf;

use semtree_lint::LintEngine;
use semtree_semantic::SemanticModel;

pub fn lint(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let parse_result = semtree_parser::Parser::parse(&source);
    let root = parse_result.syntax();
    let model = SemanticModel::analyze(&root);

    let engine = LintEngine::with_defaults();
    let result = engine.lint(&root, &model);

    if result.is_clean() {
        println!("No issues found.");
        return Ok(());
    }

    for diag in &result.diagnostics {
        let start = u32::from(diag.range.start());
        let end = u32::from(diag.range.end());
        println!(
            "{severity} [{rule}] {msg} ({start}..{end})",
            severity = diag.severity,
            rule = diag.rule,
            msg = diag.message,
        );
        if let Some(ref fix) = diag.fix {
            println!("  fix: {fix}");
        }
    }

    println!(
        "\n{} warning(s), {} error(s)",
        result.warning_count(),
        result.error_count()
    );

    Ok(())
}

use std::path::PathBuf;

use semtree_parser::Parser;

pub fn test(dir: PathBuf) -> super::Result {
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()).into());
    }

    let mut passed = 0u32;
    let mut failed = 0u32;

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt")
            || path.extension().and_then(|e| e.to_str()) == Some("src")
        {
            let source = std::fs::read_to_string(&path)?;
            let result = Parser::parse(&source);
            let reconstructed = result.syntax().text();

            if reconstructed == source {
                passed += 1;
            } else {
                failed += 1;
                eprintln!("FAIL: {} (lossless roundtrip failed)", path.display());
                if !result.errors.is_empty() {
                    for e in &result.errors {
                        eprintln!("  error: {e}");
                    }
                }
            }
        }
    }

    println!("Test results: {passed} passed, {failed} failed");
    if failed > 0 {
        Err(format!("{failed} test(s) failed").into())
    } else {
        Ok(())
    }
}

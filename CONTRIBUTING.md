# Contributing to SemTree

Thank you for your interest in contributing to SemTree! This document provides guidelines and instructions for contributing.

## Getting Started

1. **Fork** the repository
2. **Clone** your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/semtree.git
   cd semtree
   ```
3. **Build** the project:
   ```bash
   cargo build
   ```
4. **Run tests** to make sure everything works:
   ```bash
   cargo test
   ```

## Development Setup

- **Rust 1.85+** (edition 2024)
- Run `cargo build` to compile all 19 crates
- Run `cargo test` to execute the full test suite
- Run `cargo clippy` for lint checks

## How to Contribute

### Reporting Bugs

- Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md)
- Include the Rust version (`rustc --version`), OS, and steps to reproduce
- Attach the grammar file and input source if relevant

### Suggesting Features

- Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md)
- Explain the use case and how it benefits the project

### Adding a New Language Grammar

1. Create `grammars/YOUR_LANGUAGE.semtree` using the SemTree DSL
2. Add test files in `grammars/tests/`
3. Add the file extension mapping in `crates/semtree_cli/src/commands/run.rs`
4. Test with `cargo run --bin semtree -- run -g grammars/YOUR_LANGUAGE.semtree test_file.ext`

### Submitting Pull Requests

1. Create a feature branch from `master`:
   ```bash
   git checkout -b feature/your-feature
   ```
2. Make your changes with clear, focused commits
3. Ensure all tests pass: `cargo test`
4. Ensure no clippy warnings: `cargo clippy`
5. Push and open a Pull Request against `master`

## Code Style

- Follow standard Rust conventions (`rustfmt`)
- Keep functions small and focused
- Add tests for new functionality
- Don't add comments that just narrate what code does — comments should explain *why*, not *what*

## Architecture

The project is organized as a Cargo workspace with 19 crates. Each crate has a focused responsibility:

```
semtree_core       → Foundation types
semtree_lexer      → Tokenization
semtree_green      → Immutable syntax tree
semtree_red        → Navigable syntax tree
semtree_parser     → Event-based parser
semtree_grammar    → Grammar IR + DSL
semtree_runtime    → Grammar-driven parser (RD + GLR)
semtree_query      → Tree queries
semtree_ast        → Typed AST
semtree_semantic   → Symbol resolution
semtree_format     → Code formatting
semtree_lint       → Linting
semtree_ide        → IDE services
semtree_refactor   → Refactoring API
semtree_ai         → AI APIs
semtree_plugin     → Plugin system
semtree_ffi        → C FFI
semtree_cli        → CLI tool
```

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

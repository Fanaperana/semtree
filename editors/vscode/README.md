# SemTree for VS Code

A thin [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
client that connects VS Code to the SemTree language server (`semtree lsp`).

From a single `.semtree` grammar you get:

- **Semantic highlighting** (keywords, operators, strings, numbers, comments, symbols)
- **Document symbols** (outline)
- **Go-to-definition** and **find-references**
- **Document highlight** and **syntax-aware selection ranges**
- **Rename** (with prepare/validation)
- **Code actions**: extract variable, inline variable
- **Diagnostics**: parse errors + lint results
- **Formatting** and **folding ranges**

## Prerequisites

Install the `semtree` binary and make sure it is on your `PATH`:

```bash
cargo install --path crates/semtree_cli   # from the SemTree repo
semtree --version
```

## Install (from source, for development)

```bash
cd editors/vscode
npm install
```

Then open this folder in VS Code and press <kbd>F5</kbd> to launch an Extension
Development Host, or package it:

```bash
npx @vscode/vsce package
code --install-extension semtree-vscode-0.1.0.vsix
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `semtree.serverPath` | `semtree` | Path to the `semtree` binary (PATH or absolute). |
| `semtree.trace.server` | `off` | Trace LSP traffic (`off` / `messages` / `verbose`). |

## How it works

The extension runs `semtree lsp` over stdio and forwards requests for the
supported languages. Grammars are resolved by the server from a `grammars/`
folder in your workspace (or the shipped defaults). See the repository README
for grammar authoring.

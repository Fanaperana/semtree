// Minimal VS Code language client for the SemTree LSP server.
//
// It launches `semtree lsp` over stdio and connects it to the editor. No build
// step is required — this is plain JavaScript using `vscode-languageclient`.
const { workspace } = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

/** @type {import('vscode-languageclient/node').LanguageClient | undefined} */
let client;

const LANGUAGES = [
  "rust",
  "python",
  "javascript",
  "javascriptreact",
  "json",
  "css",
  "toml",
];

function activate(_context) {
  const config = workspace.getConfiguration("semtree");
  const command = config.get("serverPath") || "semtree";

  const serverOptions = {
    run: { command, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command, args: ["lsp"], transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: LANGUAGES.map((language) => ({ scheme: "file", language })),
    synchronize: {
      // Re-sync when a grammar file changes.
      fileEvents: workspace.createFileSystemWatcher("**/*.semtree"),
    },
  };

  client = new LanguageClient(
    "semtree",
    "SemTree Language Server",
    serverOptions,
    clientOptions
  );
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };

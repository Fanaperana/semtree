# Tutorial: Install SemTree

**Goal:** get a working `semtree` binary on your machine.

**Time:** ~5 minutes

**You need:** Rust 1.85+ (`rustc --version`)

---

## 1. Clone the repository

```bash
git clone https://github.com/Fanaperana/semtree.git
cd semtree
```

## 2. Install the CLI

```bash
cargo install --path crates/semtree_cli
```

This puts `semtree` in `~/.cargo/bin/`. Make sure that directory is on your `PATH`:

```bash
# macOS / Linux (zsh)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## 3. Verify

```bash
semtree --help
semtree doctor
```

You should see the command list and a health report with no fatal errors.

## Done

You now have SemTree installed. Next: [Parse your first file](02-parse-first-file.md).

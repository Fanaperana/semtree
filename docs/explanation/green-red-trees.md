# Green trees and red trees

SemTree uses the same green/red split popularized by rust-analyzer.

## Green tree

- Immutable
- No parent pointers
- Shared with `Arc` across versions of a file
- Ideal for incremental reparsing: unchanged subtrees are reused by pointer

## Red tree

- Built on demand from a green root
- Adds parent / sibling / ancestor navigation
- Carries absolute text offsets
- Cheap to recreate after an edit

## Why both

Editors need navigation (red). Incremental parsers need structural sharing (green). Keeping them separate means you don’t pay for parents in the cacheable layer.

## Losslessness

Trivia (whitespace, comments) is preserved in the tree so `root.text()` round-trips to the original source when the parse succeeds.

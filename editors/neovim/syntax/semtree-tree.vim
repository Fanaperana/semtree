" Vim syntax file for SemTree parse output (sexp-pretty and tree formats)
if exists("b:current_syntax")
    finish
endif

" ── Tree format: NodeKind@start..end "text" ──────────────────
" Branch nodes: PascalCase or snake_case names followed by @range
syn match semtreeTreeBranch   "\<[A-Z][A-Za-z0-9_]*\>\ze@"
syn match semtreeTreeLeaf     "\<[a-z][a-z0-9_]*\>\ze@"
syn match semtreeTreeRange    "@\d\+\.\.\d\+"
syn match semtreeTreeText     /"[^"]*"/

" ── S-expression format: (NodeKind [start..end] ─────────────
" Branch: (PascalCase [range]
syn match semtreeSexpBranch   "(\zs[A-Z][A-Za-z0-9_]*\>"
" Leaf: (lowercase "text")
syn match semtreeSexpLeaf     "(\zs[a-z][a-z0-9_]*\>"
syn match semtreeSexpRange    "\[\d\+\.\.\d\+\]"
syn match semtreeSexpText     /"[^"]*"/

" ── ERROR nodes (both formats) ───────────────────────────────
syn match semtreeTreeError    "\<ERROR\>"

" ── Info lines (Using grammar, Backend, error summary) ───────
syn match semtreeTreeInfo     "^Using grammar:.*$"
syn match semtreeTreeInfo     "^Backend:.*$"
syn match semtreeTreeInfo     "^--- \d\+ error.*$"
syn match semtreeTreeErrMsg   "^\s*error at.*$"

" ── Highlighting links ───────────────────────────────────────
hi def link semtreeTreeBranch   Function
hi def link semtreeTreeLeaf     Type
hi def link semtreeTreeRange    Comment
hi def link semtreeTreeText     String
hi def link semtreeSexpBranch   Function
hi def link semtreeSexpLeaf     Type
hi def link semtreeSexpRange    Comment
hi def link semtreeSexpText     String
hi def link semtreeTreeError    Error
hi def link semtreeTreeInfo     Comment
hi def link semtreeTreeErrMsg   DiagnosticError

let b:current_syntax = "semtree-tree"

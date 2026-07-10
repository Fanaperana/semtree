" Vim syntax file for SemTree inspector output
" The inspector also applies programmatic extmark highlighting, but this
" provides a baseline when extmarks haven't been applied yet.
if exists("b:current_syntax")
    finish
endif

" Branch nodes: PascalCase [range]
syn match semtreeInspBranch   "\<[A-Z][A-Za-z0-9_]*\>"
" Leaf nodes: lowercase tokens
syn match semtreeInspLeaf     "\<[a-z][a-z0-9_]*\>"
" Ranges
syn match semtreeInspRange    "\[\d\+\.\.\d\+\]"
" Quoted text
syn match semtreeInspText     /"[^"]*"/
" ERROR nodes
syn match semtreeInspError    "\<ERROR\>"

hi def link semtreeInspBranch   Function
hi def link semtreeInspLeaf     Type
hi def link semtreeInspRange    Comment
hi def link semtreeInspText     String
hi def link semtreeInspError    Error

let b:current_syntax = "semtree-inspect"

" Vim syntax file for SemTree symbols output
if exists("b:current_syntax")
    finish
endif

" Symbol kinds (function, class, method, variable, etc.)
syn match semtreeSymKind      "\<\(function\|class\|method\|variable\|constant\|module\|struct\|enum\|trait\|impl\|type\|field\|property\)\>"
" Symbol names
syn match semtreeSymName      "\S\+\ze\s*(" 
syn match semtreeSymName      "\S\+\ze\s*\[" 
" Ranges and line numbers
syn match semtreeSymRange     "\[\d\+\.\.\d\+\]"
syn match semtreeSymRange     "line \d\+"
" Info message
syn match semtreeSymInfo      "^No symbols found\.$"

hi def link semtreeSymKind    Keyword
hi def link semtreeSymName    Function
hi def link semtreeSymRange   Comment
hi def link semtreeSymInfo    WarningMsg

let b:current_syntax = "semtree-symbols"

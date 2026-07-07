" Vim syntax file for SemTree grammar DSL (.semtree)
if exists("b:current_syntax")
    finish
endif

" ── Comments ─────────────────────────────────────────────────
syn match semtreeComment    "#.*$"

" ── Top-level directives ─────────────────────────────────────
syn match semtreeDirective  "^\s*\<language\>"
syn match semtreeDirective  "^\s*\<keyword\>"
syn match semtreeDirective  "^\s*\<token\>"
syn match semtreeDirective  "^\s*\<skip\>"
syn match semtreeDirective  "^\s*\<precedence\>"

" ── Format hints ─────────────────────────────────────────────
syn match semtreeFormatHint "^\s*\<indent\>"
syn match semtreeFormatHint "^\s*\<linebreak\>"
syn match semtreeFormatHint "^\s*\<space\>"
syn match semtreeFormatHint "\<before\>"     contained containedin=semtreeFormatHint
syn match semtreeFormatHint "\<after\>"      contained containedin=semtreeFormatHint
syn match semtreeFormatHint "\<around\>"     contained containedin=semtreeFormatHint

" ── Rule definition (Name :=) ────────────────────────────────
syn match semtreeRuleName   "^\s*[A-Z][A-Za-z0-9_]*" nextgroup=semtreeAssign skipwhite
syn match semtreeAssign     ":=" contained

" ── Language name after 'language' directive ──────────────────
syn match semtreeLangName   "\<language\>\s\+\zs[a-z][a-z0-9_-]*"

" ── Keyword name after 'keyword' directive ───────────────────
syn match semtreeKeywordVal "\<keyword\>\s\+\zs\S\+"

" ── String literals ──────────────────────────────────────────
syn region semtreeString    start=+"+ end=+"+ skip=+\\"+

" ── Operators ────────────────────────────────────────────────
syn match semtreeOperator   "|"
syn match semtreeQuantifier "[?*+]"

" ── Rule references (PascalCase in rule bodies) ──────────────
syn match semtreeRuleRef    "\s\zs[A-Z][A-Za-z0-9_]*\ze[^:]" contained containedin=semtreeRuleBody

" ── Field names (name: before a reference) ───────────────────
syn match semtreeField      "[a-z][a-z0-9_]*\ze:" containedin=semtreeRuleBody

" ── Built-in type references ─────────────────────────────────
syn keyword semtreeBuiltin  Identifier Integer Float String Number

" ── Highlighting links ───────────────────────────────────────
hi def link semtreeComment      Comment
hi def link semtreeDirective    Keyword
hi def link semtreeFormatHint   PreProc
hi def link semtreeRuleName     Type
hi def link semtreeAssign       Operator
hi def link semtreeLangName     String
hi def link semtreeKeywordVal   Constant
hi def link semtreeString       String
hi def link semtreeOperator     Operator
hi def link semtreeQuantifier   Special
hi def link semtreeRuleRef      Identifier
hi def link semtreeField        Label
hi def link semtreeBuiltin      Special

let b:current_syntax = "semtree"

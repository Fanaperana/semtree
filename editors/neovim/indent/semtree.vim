" Indent rules for SemTree grammar DSL (.semtree)
if exists("b:did_indent")
    finish
endif
let b:did_indent = 1

setlocal indentexpr=SemTreeIndent()
setlocal indentkeys=o,O,<CR>
setlocal autoindent

function! SemTreeIndent()
    let prevlnum = prevnonblank(v:lnum - 1)
    if prevlnum == 0
        return 0
    endif

    let prevline = getline(prevlnum)
    let curline = getline(v:lnum)
    let ind = indent(prevlnum)

    " Indent after rule definition (Name :=)
    if prevline =~# '^\s*[A-Z][A-Za-z0-9_]*\s*:=\s*$'
        return ind + &shiftwidth
    endif

    " Stay indented for continuation lines (starting with |)
    if curline =~# '^\s*|'
        " Find the rule definition above
        let lnum = prevlnum
        while lnum > 0
            if getline(lnum) =~# '^\s*[A-Z][A-Za-z0-9_]*\s*:='
                return indent(lnum) + &shiftwidth
            endif
            let lnum = lnum - 1
        endwhile
    endif

    " De-indent when starting a new rule or directive
    if curline =~# '^\s*[A-Z][A-Za-z0-9_]*\s*:=' || curline =~# '^\s*\(language\|keyword\|token\|indent\|linebreak\|space\|skip\|precedence\)\>'
        return 0
    endif

    return ind
endfunction

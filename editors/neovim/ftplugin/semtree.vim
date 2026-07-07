" Filetype settings for SemTree grammar files (.semtree)
if exists("b:did_ftplugin")
    finish
endif
let b:did_ftplugin = 1

setlocal commentstring=#\ %s
setlocal comments=:#
setlocal shiftwidth=4
setlocal tabstop=4
setlocal expandtab
setlocal suffixesadd=.semtree

" Folding: fold rule bodies
setlocal foldmethod=indent
setlocal foldlevel=99

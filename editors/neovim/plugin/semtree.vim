" SemTree Neovim Plugin
if exists('g:loaded_semtree')
    finish
endif
let g:loaded_semtree = 1

" Register .semtree filetype
autocmd BufRead,BufNewFile *.semtree setfiletype semtree

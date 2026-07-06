; Keywords
(keyword) @keyword
"function" @keyword
"const" @keyword
"let" @keyword
"var" @keyword
"if" @keyword
"else" @keyword
"return" @keyword
"class" @keyword
"new" @keyword
"for" @keyword
"while" @keyword

; Functions
(function_decl name: (identifier) @function)
(method_def name: (identifier) @function.method)

; Strings
(string) @string

; Numbers
(integer) @number
(float) @number.float

; Operators
"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
";" @punctuation.delimiter
"," @punctuation.delimiter
":" @punctuation.delimiter

; Types
(identifier) @variable

; Comments
(comment) @comment

; Keywords
(keyword) @keyword
"fn" @keyword
"let" @keyword
"mut" @keyword
"const" @keyword
"static" @keyword
"if" @keyword
"else" @keyword
"match" @keyword
"return" @keyword
"struct" @keyword
"enum" @keyword
"impl" @keyword
"trait" @keyword
"pub" @keyword
"use" @keyword
"mod" @keyword
"crate" @keyword
"self" @keyword
"super" @keyword
"for" @keyword
"while" @keyword
"loop" @keyword
"break" @keyword
"continue" @keyword
"async" @keyword
"await" @keyword
"where" @keyword
"type" @keyword
"unsafe" @keyword
"move" @keyword
"ref" @keyword
"as" @keyword

; Functions
(function_item name: (identifier) @function)
(method_decl name: (identifier) @function.method)
(macro_invocation name: (identifier) @function.macro)

; Types
(type_identifier) @type
(primitive_type) @type.builtin
(struct_item name: (identifier) @type)
(enum_item name: (identifier) @type)
(trait_item name: (identifier) @type)

; Strings
(string_literal) @string
(char_literal) @character
(raw_string_literal) @string

; Numbers
(integer_literal) @number
(float_literal) @number.float

; Booleans
"true" @boolean
"false" @boolean

; Operators
"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"&" @operator
"|" @operator
"!" @operator
"<" @operator
">" @operator
"->" @operator
"=>" @operator
"::" @operator

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

; Lifetimes
(lifetime) @label

; Attributes
(attribute) @attribute

; Variables
(identifier) @variable

; Comments
(line_comment) @comment
(block_comment) @comment

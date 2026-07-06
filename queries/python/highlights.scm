; Keywords
(keyword) @keyword
"def" @keyword
"class" @keyword
"if" @keyword
"elif" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"return" @keyword
"import" @keyword
"from" @keyword
"as" @keyword
"with" @keyword
"try" @keyword
"except" @keyword
"finally" @keyword
"raise" @keyword
"pass" @keyword
"break" @keyword
"continue" @keyword
"yield" @keyword
"lambda" @keyword
"and" @keyword
"or" @keyword
"not" @keyword
"in" @keyword
"is" @keyword
"global" @keyword
"nonlocal" @keyword
"async" @keyword
"await" @keyword

; Functions
(function_definition name: (identifier) @function)
(decorated_definition definition: (function_definition name: (identifier) @function))
(call function: (identifier) @function.call)

; Classes
(class_definition name: (identifier) @type)

; Strings
(string) @string
(interpolation) @string.special

; Numbers
(integer) @number
(float) @number.float

; Booleans
"True" @boolean
"False" @boolean
"None" @constant.builtin

; Operators
"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"**" @operator
"//" @operator
"%" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
":" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Decorators
(decorator) @attribute

; Variables
(identifier) @variable

; Parameters
(parameters (identifier) @variable.parameter)

; Comments
(comment) @comment

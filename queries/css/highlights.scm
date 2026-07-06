; At-rules
(at_keyword) @keyword
"@media" @keyword
"@import" @keyword
"@keyframes" @keyword
"@font-face" @keyword
"@supports" @keyword
"@charset" @keyword

; Selectors
(class_selector) @type
(id_selector) @constant
(tag_name) @tag
(universal_selector) @operator
(pseudo_class_selector) @attribute
(pseudo_element_selector) @attribute

; Properties
(property_name) @property

; Values
(plain_value) @string
(color_value) @constant
(integer_value) @number
(float_value) @number.float
(string_value) @string
(url) @string.special

; Units
(unit) @type

; Functions
(function_name) @function

; Operators
":" @operator
"+" @operator
"~" @operator
">" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
";" @punctuation.delimiter
"," @punctuation.delimiter

; Important
"!important" @keyword

; Comments
(comment) @comment

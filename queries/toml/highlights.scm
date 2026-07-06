; Keys
(bare_key) @property
(dotted_key) @property
(quoted_key) @property

; Tables
(table (bare_key) @type)
(table_array_element (bare_key) @type)

; Strings
(string) @string
(multiline_string) @string

; Numbers
(integer) @number
(float) @number.float

; Booleans
(boolean) @boolean

; Dates
(local_date) @string.special
(local_time) @string.special
(local_date_time) @string.special
(offset_date_time) @string.special

; Operators
"=" @operator

; Punctuation
"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"," @punctuation.delimiter
"." @punctuation.delimiter

; Comments
(comment) @comment

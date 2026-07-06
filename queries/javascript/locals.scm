; Scopes
(function_decl) @scope
(block) @scope

; Definitions
(variable_decl name: (identifier) @definition.var)
(function_decl name: (identifier) @definition.function)
(param name: (identifier) @definition.parameter)

; References
(identifier) @reference

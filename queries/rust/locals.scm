; Scopes
(function_item) @scope
(block) @scope
(impl_item) @scope
(closure_expression) @scope

; Definitions
(let_declaration pattern: (identifier) @definition.var)
(function_item name: (identifier) @definition.function)
(parameter pattern: (identifier) @definition.parameter)
(struct_item name: (identifier) @definition.type)
(enum_item name: (identifier) @definition.type)
(type_item name: (identifier) @definition.type)
(const_item name: (identifier) @definition.var)
(static_item name: (identifier) @definition.var)

; References
(identifier) @reference
(type_identifier) @reference

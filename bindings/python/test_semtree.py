import semtree

parser = semtree.Parser(open("../../grammars/json.semtree").read())
tree = parser.parse('{"name": "test", "value": 42}')
print(tree.to_sexp())
print(tree.root_node().kind_name())
print(tree.root_node().child_count())

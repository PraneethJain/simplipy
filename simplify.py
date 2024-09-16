import ast
import astor


class ASTModifier(ast.NodeTransformer):
    def visit_For(self, node):
        self.generic_visit(node)
        if not isinstance(node.body[-1], ast.Continue):
            node.body.append(ast.Continue())
        return node

    def visit_While(self, node):
        self.generic_visit(node)
        if not isinstance(node.body[-1], ast.Continue):
            node.body.append(ast.Continue())
        return node

    def visit_FunctionDef(self, node):
        self.generic_visit(node)
        if not node.body or not isinstance(node.body[-1], ast.Return):
            node.body.append(ast.Return(value=None))
        return node


if __name__ == "__main__":
    with open("main.py", "r") as file:
        source = file.read()
    tree = ast.parse(source)
    simplifier = ASTModifier()
    simplified_tree = simplifier.visit(tree)
    simplified_source = astor.to_source(simplified_tree)
    with open("simplified_main.py", "w") as file:
        file.write(simplified_source)

import ast
import astor

from rich import print as rprint


class CodeAnalyzer(ast.NodeVisitor):
    def __init__(self):
        self.statements = {}
        self.next = {}
        self.true = {}
        self.false = {}
        self.decvars = {"global": set()}

        self._parent_map = {}
        self._loop_stack = []
        self._current_function = None
        self._current_class = None

    def visit(self, node):
        for child in ast.iter_child_nodes(node):
            self._parent_map[child] = node
        return super().visit(node)

    def find_next_line(self, node):
        while node:
            parent = self._parent_map.get(node)
            if not parent:
                return None

            for field, value in ast.iter_fields(parent):
                if isinstance(value, list) and node in value:
                    index = value.index(node)
                    if index + 1 < len(value):
                        return value[index + 1].lineno
                elif value == node:
                    break
            node = parent

        return None

    def add_statement(self, node, stmt_type: str, **kwargs) -> None:
        if hasattr(node, "lineno"):
            self.statements[node.lineno] = {
                "type": stmt_type,
                "col_offset": node.col_offset if hasattr(node, "col_offset") else None,
                **kwargs,
            }
            next_line = self.find_next_line(node)
            if next_line:
                self.next[node.lineno] = next_line

    def add_variable(self, name: str) -> None:
        if self._current_class is None and self._current_function is None:
            self.decvars["global"].add(name)
        elif self._current_class is None:
            self.decvars[self._current_function.lineno].add(name)
        else:
            self.decvars[self._current_class.lineno].add(name)

    def visit_Assign(self, node):
        targets = [astor.to_source(target).strip() for target in node.targets]
        value = astor.to_source(node.value).strip()
        if isinstance(node.value, ast.Call):
            func_name = astor.to_source(node.value.func).strip()
            args = [astor.to_source(arg).strip() for arg in node.value.args]
            keywords = {
                kw.arg: astor.to_source(kw.value).strip() for kw in node.value.keywords
            }

            self.add_statement(
                node,
                "AssignmentWithFunctionCall",
                targets=targets,
                value=value,
                func_name=func_name,
                func_args=args,
                func_keywords=keywords,
            )
        else:
            self.add_statement(node, "Assignment", targets=targets, value=value)

        for target in node.targets:
            if isinstance(target, ast.Name):
                self.add_variable(target.id)

    def visit_AugAssign(self, node):
        target = astor.to_source(node.target).strip()
        op = type(node.op).__name__
        value = astor.to_source(node.value).strip()
        self.add_statement(
            node, "AugmentedAssignment", target=target, op=op, value=value
        )
        if isinstance(node.target, ast.Name):
            self.add_variable(node.target.id)

    def visit_FunctionDef(self, node):
        args = [arg.arg for arg in node.args.args]
        defaults = [astor.to_source(default).strip() for default in node.args.defaults]
        decorators = [astor.to_source(d).strip() for d in node.decorator_list]
        self.add_statement(
            node,
            "FunctionDefinition",
            name=node.name,
            args=args,
            defaults=defaults,
            decorators=decorators,
            body_line_no=node.body[0].lineno,
        )

        self.add_variable(node.name)

        self._current_function = node
        self.decvars[node.lineno] = set(args)

        for item in node.body:
            self.visit(item)

        self._current_function = None

    def visit_Return(self, node):
        value = astor.to_source(node.value).strip() if node.value else None
        self.add_statement(node, "Return", value=value)

    def visit_Call(self, node):
        func = astor.to_source(node.func).strip()
        args = [astor.to_source(arg).strip() for arg in node.args]
        keywords = {kw.arg: astor.to_source(kw.value).strip() for kw in node.keywords}
        self.add_statement(
            node, "FunctionCall", func=func, args=args, keywords=keywords
        )

    def visit_If(self, node):
        test = astor.to_source(node.test).strip()
        self.add_statement(node, "If", test=test)
        if node.body:
            self.true[node.lineno] = node.body[0].lineno
        if node.orelse:
            self.false[node.lineno] = node.orelse[0].lineno
        else:
            next_line = self.find_next_line(node)
            if next_line:
                self.false[node.lineno] = next_line
        for item in node.body + node.orelse:
            self.visit(item)

    def visit_For(self, node):
        self._loop_stack.append(node.lineno)
        target = astor.to_source(node.target).strip()
        iter = astor.to_source(node.iter).strip()
        self.add_statement(node, "For", target=target, iter=iter)
        # if node.body:
        #     self.true[node.lineno] = node.body[0].lineno
        # next_line = self.find_next_line(node)
        # if next_line:
        #     self.false[node.lineno] = next_line
        for item in node.body:
            self.visit(item)
        self._loop_stack.pop()
        if isinstance(node.target, ast.Name):
            self.add_variable(node.target.id)

    def visit_While(self, node):
        self._loop_stack.append(node.lineno)
        test = astor.to_source(node.test).strip()
        self.add_statement(node, "While", test=test)
        if node.body:
            self.true[node.lineno] = node.body[0].lineno
        next_line = self.find_next_line(node)
        if next_line:
            self.false[node.lineno] = next_line
        for item in node.body:
            self.visit(item)
        self._loop_stack.pop()

    def visit_Break(self, node):
        self.add_statement(node, "Break")
        if self._loop_stack:
            self.next[node.lineno] = self.false[self._loop_stack[-1]]

    def visit_Continue(self, node):
        self.add_statement(node, "Continue")
        if self._loop_stack:
            self.next[node.lineno] = self._loop_stack[-1]

    def visit_Import(self, node):
        names = [alias.name for alias in node.names]
        self.add_statement(node, "Import", names=names)
        for alias in node.names:
            self.add_variable(alias.asname or alias.name.split(".")[0])

    def visit_ImportFrom(self, node):
        module = node.module
        names = [alias.name for alias in node.names]
        self.add_statement(node, "ImportFrom", module=module, names=names)
        for alias in node.names:
            self.add_variable(alias.asname or alias.name)

    def visit_ClassDef(self, node):
        bases = [astor.to_source(base).strip() for base in node.bases]

        decorators = [astor.to_source(d).strip() for d in node.decorator_list]

        keywords = {kw.arg: astor.to_source(kw.value).strip() for kw in node.keywords}

        self.add_statement(
            node,
            "ClassDefinition",
            name=node.name,
            bases=bases,
            decorators=decorators,
            keywords=keywords,
        )
        self.add_variable(node.name)

        self._current_class = node
        self.decvars[node.lineno] = set()

        for item in node.body:
            self.visit(item)

        self._current_class = None

    def visit_Raise(self, node):
        exc = astor.to_source(node.exc).strip() if node.exc else None
        self.add_statement(node, "Raise", exc=exc)

    def visit_Try(self, node):
        self.add_statement(node, "Try")
        for item in node.body + node.handlers + node.finalbody:
            self.visit(item)

    def visit_With(self, node):
        items = [astor.to_source(item).strip() for item in node.items]
        self.add_statement(node, "With", items=items)
        for item in node.body:
            self.visit(item)

    def visit_Expr(self, node):
        value = astor.to_source(node.value).strip()
        self.add_statement(node, "Expression", value=value)


def analyze_python_file(file_path):
    with open(file_path, "r") as file:
        source = file.read()

    tree = ast.parse(source)

    analyzer = CodeAnalyzer()
    analyzer.visit(tree)

    rprint(analyzer.decvars)
    rprint(analyzer.false)

    return analyzer


if __name__ == "__main__":
    results = analyze_python_file("test.py")

    # for line_num, info in sorted(results.items()):
    #     print(f"Line {line_num}: {info['type']}")
    #     for key, value in info.items():
    #         if key not in ("type", "col_offset"):
    #             print(f"  {key}: {value}")
    #     print()

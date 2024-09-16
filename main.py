from copy import deepcopy
from functools import reduce
import operator
from rich import print as rprint
import utils


class State:
    def __init__(self) -> None:
        self.lineno = min(static.statements.keys())
        self.store = []
        self.nested_environment = []
        self.stack = []

        self.initialize_new_env(static.decvars["global"])

    def initialize_new_env(self, decvars: set[str]) -> None:
        n = len(self.store)
        env = {decvar: i + n for i, decvar in enumerate(decvars)}
        self.store.extend(["💀"] * len(decvars))
        self.nested_environment.append(env)

    def tick(self) -> None:
        stmt = static.statements[self.lineno]
        match stmt["type"]:

            case "Assignment":
                assert len(stmt["targets"]) == 1
                var_name = stmt["targets"][0]
                value = self.eval(stmt["value"])
                self.update_var(var_name, value)
                self.lineno = static.next[self.lineno]

            case "FunctionDefinition":
                var_name = stmt["name"]
                closure = (self.lineno, deepcopy(self.nested_environment))
                self.update_var(var_name, closure)
                self.lineno = static.next[self.lineno]

            case "AssignmentWithFunctionCall":
                assert len(stmt["targets"]) == 1
                func_name = stmt["func_name"]
                lineno, nested_env = deepcopy(self.lookup(func_name))

                vals = list(map(self.eval, stmt["func_args"]))
                formals = static.statements[lineno]["args"]
                assert len(formals) == len(vals)

                self.stack.append((self.lineno, deepcopy(self.nested_environment)))
                self.nested_environment = nested_env
                self.initialize_new_env(static.decvars[lineno])
                for formal, val in zip(formals, vals):
                    self.update_var(formal, val)

                self.lineno = static.statements[lineno]["body_line_no"]

            case "Return":
                value = self.eval(stmt["value"])
                lineno, nested_env = self.stack.pop()
                var_name = static.statements[lineno]["targets"][0]
                self.nested_environment = nested_env
                self.update_var(var_name, value)
                self.lineno = static.next[lineno]

            case "If" | "While":
                condition = stmt["test"]
                if self.eval(condition):
                    self.lineno = static.true[self.lineno]
                else:
                    self.lineno = static.false[self.lineno]

            case "Continue" | "Break":
                self.lineno = static.next[self.lineno]

            case "Expression":
                # Remove this later maybe? Just for prints
                self.eval(stmt["value"])
                self.lineno = static.next[self.lineno]

            case other:
                raise NotImplementedError(f"{other} not implemented yet")

        var_val_mapping = self._get_var_val_mapping()
        rprint(self.lineno, var_val_mapping)

    def _get_var_val_mapping(self):
        mapping = []
        for env in self.nested_environment:
            mapping.append({var_name: self.store[idx] for var_name, idx in env.items()})
        return mapping

    def lookup(self, var_name: str):
        for env in reversed(self.nested_environment):
            if var_name in env:
                return self.store[env[var_name]]
        raise LookupError()

    def get_rib(self, var_name: str) -> dict[str, int]:
        for env in reversed(self.nested_environment):
            if var_name in env:
                return env
        raise LookupError()

    def update_var(self, var_name: str, value) -> None:
        env = self.get_rib(var_name)
        self.store[env[var_name]] = value

    def eval(self, expr: str):
        env = reduce(operator.ior, self.nested_environment, {})
        env_to_val = {k: self.store[v] for k, v in env.items() if v is not None}
        result = eval(expr, env_to_val)
        return result


if __name__ == "__main__":
    filename = "test.py"
    static = utils.analyze_python_file(filename)
    state = State()

    while True:
        input()
        state.tick()

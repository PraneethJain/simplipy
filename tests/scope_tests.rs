use rustpython_parser::{ast::bigint::BigInt, parse, source_code::LineIndex, Mode};
use simplipy::{
    self,
    datatypes::StorableValue,
    preprocess::preprocess_module,
    state::{init_state, is_fixed_point, tick},
    utils::lookup,
};

mod common;

#[test]
fn test_simple_nesting() {
    let source = r#"
def make_adder(x):
    def adder(y):
        return x + y
    return adder

inc = make_adder(1)
plus10 = make_adder(10)

a = inc(1)
b = plus10(-2)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(2))),
        ("b", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_extra_nesting() {
    let source = r#"
def make_adder2(x):
    def extra():
        def adder(y):
            return x + y
        return adder
    temp = extra()
    return temp

inc = make_adder2(1)
plus10 = make_adder2(10)

a = inc(1)
b = plus10(-2)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(2))),
        ("b", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_simple_and_rebinding() {
    let source = r#"
def make_adder3(x):
    def adder(y):
        return x + y
    x = x + 1
    return adder

inc = make_adder3(0)
plus10 = make_adder3(9)

a = inc(1)
b = plus10(-2)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(2))),
        ("b", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_nesting_global_nofree() {
    let source = r#"
def make_adder4(): # XXX add exta level of indirection
    def nest():
        def nest():
            def adder(y):
                return global_x + y # check that plain old globals work
            return adder
        temp = nest()
        return temp
    temp = nest()
    return temp

global_x = 1
adder = make_adder4()
a = adder(1)

global_x = 10
b = adder(-2)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(2))),
        ("b", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_nesting_plus_freeref_to_global() {
    let source = r#"
def make_adder6(x):
    global global_nest_x
    def adder(y):
        return global_nest_x + y
    global_nest_x = x
    return adder

inc = make_adder6(1)
plus10 = make_adder6(10)

a = inc(1)
b = plus10(-2)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(11))),
        ("b", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_nearest_enclosing_scope() {
    let source = r#"
def f(x):
    def g(y):
        x = 42
        def h(z):
            return x + z
        return h
    temp = g(2)
    return temp

test_func = f(10)
a = test_func(5)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(47))));
}

#[test]
fn test_mixed_freevars_and_cellvars() {
    let source = r#"
def identity(x):
    return x

def f(x, y, z):
    def g(a, b, c):
        a = a + x
        def h():
            temp = identity(z * (b + y))
            return temp
        y = c + z
        return h
    return g

g = f(1, 2, 3)
h = g(2, 4, 6)

a = h()

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(39))));
}

#[test]
fn test_freevar_in_method() {
    let source = r#"
def test():
    method_and_var = "var"
    class Test:
        def __init__(self):
            return self
        def method_and_var(self):
            return "method"
        def test(self):
            return method_and_var
        def actual_global(self):
            return "global"
    temp = Test()
    return temp

t = test()
a = t.test()
b = t.method_and_var()
c = t.actual_global()

method_and_var = "var"
class Test:
    def __init__(self):
        return self
    def method_and_var(self):
        return "method"
    def test(self):
        return method_and_var
    def actual_global(self):
        return "global"

t = Test()
d = t.test()
e = t.method_and_var()
f = t.actual_global()

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::String(String::from("var"))),
        ("b", StorableValue::String(String::from("method"))),
        ("c", StorableValue::String(String::from("global"))),
        ("d", StorableValue::String(String::from("var"))),
        ("e", StorableValue::String(String::from("method"))),
        ("f", StorableValue::String(String::from("global"))),
    );
}

#[test]
fn test_recursion() {
    let source = r#"
def f(x):
    def fact(n):
        if n == 0:
            return 1
        else:
            temp = fact(n - 1)
            return n * temp
    temp = fact(x)
    return temp

a = f(6)

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(720))),);
}

#[test]
fn test_class_vs_lexical() {
    let source = r#"
def f():
    return 1

class A:
    x = f()

    def f(self):
        return 2

    y = f(10)

x = A.x
y = A.y

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(state, ("x", StorableValue::Int(BigInt::from(1))),);
    lookup_and_assert!(state, ("y", StorableValue::Int(BigInt::from(2))),);
}

#[test]
fn test_global() {
    let source = r#"
def g():
    x = 4
    def f():
        global x
        x = 2
        return 5
    return f

f = g()
a = f()    

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    while !is_fixed_point(&state, &static_info) {
        state = tick(state, &static_info).unwrap();
    }

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(5))),
        ("x", StorableValue::Int(BigInt::from(2)))
    );
}

#[test]
fn test_unbound_local_1() {
    let source = r#"
def errorInOuter():
    x = y
    def inner():
        return y
    y = 1
    return None

def errorInInner():
    def inner():
        return y
    x = inner()
    y = 1
    return None

_ = errorInOuter()

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    let mut error = false;
    while !is_fixed_point(&state, &static_info) {
        let temp = tick(state, &static_info);
        if temp == None {
            error = true;
            break;
        }
        state = temp.unwrap()
    }

    assert!(error);
}

#[test]
fn test_unbound_local_2() {
    let source = r#"
def errorInOuter():
    x = y
    def inner():
        return y
    y = 1
    return None

def errorInInner():
    def inner():
        return y
    x = inner()
    y = 1
    return None

_ = errorInInner()

pass
"#;

    let ast = parse(source, Mode::Module, "<embedded>").unwrap();
    let line_index = LineIndex::from_source_text(source);
    let module = ast.as_module().unwrap();
    let static_info = preprocess_module(module, &line_index, &source);
    let mut state = init_state(&static_info);

    let mut error = false;
    while !is_fixed_point(&state, &static_info) {
        let temp = tick(state, &static_info);
        if temp == None {
            error = true;
            break;
        }
        state = temp.unwrap()
    }

    assert!(error);
}

use rustpython_parser::{ast::bigint::BigInt, parse, source_code::LineIndex, Mode};
use simplipy::{
    self,
    datatypes::StorableValue,
    preprocess::preprocess_module,
    state::{init_state, is_fixed_point, tick},
    utils::lookup,
};

#[test]
fn test_recursion() {
    let source = r#"
i = 0
s = 1.0
while i < 3:
    s = s + 5.0
    i = i + 1
    continue

def fib(x):
    if x == 0:
        return 0
    if x == 1:
        return 1
    a = fib(x-1)
    b = fib(x-2)
    return a + b

z = fib(5)

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

    let z = lookup("z", &state.env, &state.store).unwrap();
    let i = lookup("i", &state.env, &state.store).unwrap();

    assert_eq!(*z, StorableValue::Int(BigInt::from(5)));
    assert_eq!(*i, StorableValue::Int(BigInt::from(3)));
}

#[test]
fn test_higher_order_1() {
    let source = r#"
def f(x):
    def g(y):
        return x + y
    return g

a = f(2)
b = a(3)

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

    let b = lookup("b", &state.env, &state.store).unwrap();

    assert_eq!(*b, StorableValue::Int(BigInt::from(5)));
}

#[test]
fn test_higher_order_2() {
    let source = r#"
x = 3

def y():
    def x(x):
        return x + 1
    return x

def f():
    z = y()
    zz = z(3)
    return zz

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

    let a = lookup("a", &state.env, &state.store).unwrap();

    assert_eq!(*a, StorableValue::Int(BigInt::from(4)));
}

#[test]
fn test_class() {
    let source = r#"
x = 3
y = 5

class A:
    x = x + 1
    y = 6

a = A.x + y
b = A.y + x
c = x

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

    let a = lookup("a", &state.env, &state.store).unwrap();
    let b = lookup("b", &state.env, &state.store).unwrap();
    let c = lookup("c", &state.env, &state.store).unwrap();

    assert_eq!(*a, StorableValue::Int(BigInt::from(9)));
    assert_eq!(*b, StorableValue::Int(BigInt::from(9)));
    assert_eq!(*c, StorableValue::Int(BigInt::from(3)));
}

#[test]
fn test_class_sharing() {
    let source = r#"
x = 3
y = 5

class A:
    x = x + 1
    y = 6
    z = x + 2

B = A
B.x = 10

a = A.x + y
b = B.y + x
c = x
d = A.z

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

    let a = lookup("a", &state.env, &state.store).unwrap();
    let b = lookup("b", &state.env, &state.store).unwrap();
    let c = lookup("c", &state.env, &state.store).unwrap();
    let d = lookup("d", &state.env, &state.store).unwrap();

    assert_eq!(*a, StorableValue::Int(BigInt::from(15)));
    assert_eq!(*b, StorableValue::Int(BigInt::from(9)));
    assert_eq!(*c, StorableValue::Int(BigInt::from(3)));
    assert_eq!(*d, StorableValue::Int(BigInt::from(6)));
}

#[test]
fn test_object() {
    let source = r#"
class A:
    x = 1
    y = 2

    def __init__(self, x, y):
        self.x = x
        self.y = y
        return self

a = A(3, 4)
x = a.x + A.x
y = a.y + A.y
z = a.x + A.y

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

    let x = lookup("x", &state.env, &state.store).unwrap();
    let y = lookup("y", &state.env, &state.store).unwrap();
    let z = lookup("z", &state.env, &state.store).unwrap();

    assert_eq!(*x, StorableValue::Int(BigInt::from(4)));
    assert_eq!(*y, StorableValue::Int(BigInt::from(6)));
    assert_eq!(*z, StorableValue::Int(BigInt::from(5)));
}

#[test]
fn test_class_scope() {
    let source = r#"
class B:
    x = 3

class A:
    class B:
        x = 4
        y = B.x
    c = B.x
    d = B.y 

a = A.c
b = A.d

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

    let a = lookup("a", &state.env, &state.store).unwrap();
    let b = lookup("b", &state.env, &state.store).unwrap();

    assert_eq!(*a, StorableValue::Int(BigInt::from(4)));
    assert_eq!(*b, StorableValue::Int(BigInt::from(3)));
}

#[test]
fn nested_classes() {
    let source = r#"
class A:
    class B:
        class C:
            x = 3
x = A.B
y = x.C
z = y.x

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

    let z = lookup("z", &state.env, &state.store).unwrap();

    assert_eq!(*z, StorableValue::Int(BigInt::from(3)));
}

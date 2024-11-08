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
fn test_integer_operations() {
    let source = r#"
a = 10
b = 5
add = a + b
sub = a - b
mul = a * b
div = a / b
mod = a % b
neg = -a

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
        ("add", StorableValue::Int(BigInt::from(15))),
        ("sub", StorableValue::Int(BigInt::from(5))),
        ("mul", StorableValue::Int(BigInt::from(50))),
        ("div", StorableValue::Int(BigInt::from(2))),
        ("mod", StorableValue::Int(BigInt::from(0))),
        ("neg", StorableValue::Int(BigInt::from(-10))),
    );
}

#[test]
fn test_float_operations() {
    let source = r#"
x = 3.14
y = 2.0
add = x + y
sub = x - y
mul = x * y
div = x / y
floor = x // y
neg = -x

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
        ("add", StorableValue::Float(3.14 + 2.0)),
        ("sub", StorableValue::Float(3.14 - 2.0)),
        ("mul", StorableValue::Float(6.28)),
        ("div", StorableValue::Float(1.57)),
        ("floor", StorableValue::Float(1.0)),
        ("neg", StorableValue::Float(-3.14)),
    );
}

#[test]
fn test_boolean_operations() {
    let source = r#"
a = True
b = False
and_op = a and b
or_op = a or b
not_a = not a
not_b = not b
complex_bool = (a or b) and not (a and b)
chained_and = True and True and False
chained_or = False or False or True

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
        ("and_op", StorableValue::Bool(false)),
        ("or_op", StorableValue::Bool(true)),
        ("not_a", StorableValue::Bool(false)),
        ("not_b", StorableValue::Bool(true)),
        ("complex_bool", StorableValue::Bool(true)),
        ("chained_and", StorableValue::Bool(false)),
        ("chained_or", StorableValue::Bool(true))
    );
}

#[test]
fn test_string_operations() {
    let source = r#"
str1 = "Hello"
str2 = " World"
concat = str1 + str2

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
        ("concat", StorableValue::String("Hello World".to_string())),
    );
}

#[test]
fn test_comparison_operations() {
    let source = r#"
a = 10
b = 20
c = 10.0
d = "hello"
e = "hello"

eq_int = a == a
neq_int = a != b
lt = a < b
gt = b > a
lte = a <= b
gte = b >= a
str_eq = d == e
multi_comp = a < b < 30

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
        ("eq_int", StorableValue::Bool(true)),
        ("neq_int", StorableValue::Bool(true)),
        ("lt", StorableValue::Bool(true)),
        ("gt", StorableValue::Bool(true)),
        ("lte", StorableValue::Bool(true)),
        ("gte", StorableValue::Bool(true)),
        ("str_eq", StorableValue::Bool(true)),
        ("multi_comp", StorableValue::Bool(true))
    );
}

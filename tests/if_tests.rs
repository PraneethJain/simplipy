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
fn test_simple_if() {
    let source = r#"
x = 1
if True:
    y = 2
    z = 3
a = y + z

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
        ("x", StorableValue::Int(BigInt::from(1))),
        ("y", StorableValue::Int(BigInt::from(2))),
        ("z", StorableValue::Int(BigInt::from(3))),
        ("a", StorableValue::Int(BigInt::from(5)))
    );
}

#[test]
fn test_if_else_basic() {
    let source = r#"
condition = False
if condition:
    x = 1
    y = 2
else:
    x = 3
    y = 4
result = x + y

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
        ("x", StorableValue::Int(BigInt::from(3))),
        ("y", StorableValue::Int(BigInt::from(4))),
        ("result", StorableValue::Int(BigInt::from(7)))
    );
}

#[test]
fn test_nested_if() {
    let source = r#"
outer = True
inner = False

if outer:
    x = 1
    if inner:
        y = 2
    y = 3
    z = 4

result = x + y + z

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
        ("x", StorableValue::Int(BigInt::from(1))),
        ("y", StorableValue::Int(BigInt::from(3))),
        ("z", StorableValue::Int(BigInt::from(4))),
        ("result", StorableValue::Int(BigInt::from(8)))
    );
}

#[test]
fn test_nested_if_else() {
    let source = r#"
outer = True
inner = False

if outer:
    x = 1
    if inner:
        y = 2
        z = 3
    else:
        y = 4
        z = 5
    w = 6
else:
    x = 7
    y = 8
    z = 9
    w = 10

result = x + y + z + w

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
        ("x", StorableValue::Int(BigInt::from(1))),
        ("y", StorableValue::Int(BigInt::from(4))),
        ("z", StorableValue::Int(BigInt::from(5))),
        ("w", StorableValue::Int(BigInt::from(6))),
        ("result", StorableValue::Int(BigInt::from(16)))
    );
}

#[test]
fn test_complex_nested_scopes() {
    let source = r#"
a = 1
if True:
    b = 2
    if False:
        c = 3
        d = 4
    else:
        if True:
            c = 5
            if True:
                d = 6
            else:
                d = 7
        else:
            c = 8
            d = 9

result = a + b + c + d

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
        ("a", StorableValue::Int(BigInt::from(1))),
        ("b", StorableValue::Int(BigInt::from(2))),
        ("c", StorableValue::Int(BigInt::from(5))),
        ("d", StorableValue::Int(BigInt::from(6))),
        ("result", StorableValue::Int(BigInt::from(14)))
    );
}

#[test]
fn test_scope_shadowing() {
    let source = r#"
x = 1
if True:
    x = 2
    if True:
        x = 3
        y = 4
    y = 5
else:
    x = 6
    y = 7

result = x + y

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
        ("x", StorableValue::Int(BigInt::from(3))),
        ("y", StorableValue::Int(BigInt::from(5))),
        ("result", StorableValue::Int(BigInt::from(8)))
    );
}

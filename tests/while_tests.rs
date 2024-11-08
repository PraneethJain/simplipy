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
fn test_simple_while() {
    let source = r#"
count = 0
sum = 0
while count < 3:
    sum = sum + count
    count = count + 1
    continue

result = sum
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
        ("count", StorableValue::Int(BigInt::from(3))),
        ("sum", StorableValue::Int(BigInt::from(3))),
        ("result", StorableValue::Int(BigInt::from(3)))
    );
}

#[test]
fn test_while_with_break() {
    let source = r#"
count = 0
sum = 0
while True:
    if count >= 3:
        break
    sum = sum + count
    count = count + 1
    continue

result = sum
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
        ("count", StorableValue::Int(BigInt::from(3))),
        ("sum", StorableValue::Int(BigInt::from(3))),
        ("result", StorableValue::Int(BigInt::from(3)))
    );
}

#[test]
fn test_nested_while() {
    let source = r#"
outer = 0
inner = 0
total = 0

while outer < 2:
    while inner < 2:
        total = total + 1
        inner = inner + 1
        continue
    outer = outer + 1
    inner = 0
    continue

result = total
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
        ("outer", StorableValue::Int(BigInt::from(2))),
        ("inner", StorableValue::Int(BigInt::from(0))),
        ("total", StorableValue::Int(BigInt::from(4))),
        ("result", StorableValue::Int(BigInt::from(4)))
    );
}

#[test]
fn test_while_with_continue_condition() {
    let source = r#"
count = 0
sum = 0
while count < 5:
    count = count + 1
    if count % 2 == 0:
        continue
    sum = sum + count
    continue

result = sum  # Should sum only odd numbers: 1 + 3 + 5
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
        ("count", StorableValue::Int(BigInt::from(5))),
        ("sum", StorableValue::Int(BigInt::from(9))),
        ("result", StorableValue::Int(BigInt::from(9)))
    );
}

#[test]
fn test_nested_while_with_break() {
    let source = r#"
outer = 0
inner = 0
total = 0

while outer < 3:
    while inner < 3:
        if inner == 1 and outer == 1:
            break
        total = total + 1
        inner = inner + 1
        continue
    outer = outer + 1
    inner = 0
    continue

result = total
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
        ("outer", StorableValue::Int(BigInt::from(3))),
        ("inner", StorableValue::Int(BigInt::from(0))),
        ("total", StorableValue::Int(BigInt::from(7))),
        ("result", StorableValue::Int(BigInt::from(7)))
    );
}

#[test]
fn test_while_with_multiple_breaks() {
    let source = r#"
x = 0
y = 0
total = 0

while x < 5:
    y = 0
    while y < 5:
        if y == 3:
            break
        if x == 3:
            break
        total = total + 1
        y = y + 1
        continue
    if x == 3:
        break
    x = x + 1
    continue

result = total
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
        ("y", StorableValue::Int(BigInt::from(0))),
        ("total", StorableValue::Int(BigInt::from(9))),
        ("result", StorableValue::Int(BigInt::from(9)))
    );
}

#[test]
fn test_while_variable_scope() {
    let source = r#"
x = 0
total = 0

while x < 3:
    y = x * 2
    while y > 0:
        z = 1
        total = total + z
        y = y - 1
        continue
    x = x + 1
    continue

result = total
outer_y = y  # Should be 0
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
        ("total", StorableValue::Int(BigInt::from(6))),
        ("result", StorableValue::Int(BigInt::from(6))),
        ("outer_y", StorableValue::Int(BigInt::from(0)))
    );
}

#[test]
fn test_while_with_break_continue_mix() {
    let source = r#"
x = 0
total = 0

while x < 5:
    if x == 0:
        x = x + 1
        continue
    if x == 2:
        x = x + 2
        continue
    if x == 4:
        break
    total = total + x
    x = x + 1
    continue

result = total
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
        ("x", StorableValue::Int(BigInt::from(4))),
        ("total", StorableValue::Int(BigInt::from(1))),
        ("result", StorableValue::Int(BigInt::from(1)))
    );
}

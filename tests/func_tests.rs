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
fn test_basic_function() {
    let source = r#"
def add(x, y):
    result = x + y
    return result

a = add(5, 3)
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(8))));
}

#[test]
fn test_nested_function() {
    let source = r#"
def outer(x):
    def inner(y):
        result = x + y
        return result
    return inner

fn = outer(10)
a = fn(5)
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(15))));
}

#[test]
fn test_triple_nested_function() {
    let source = r#"
def level1(x):
    def level2(y):
        def level3(z):
            result = x + y + z
            return result
        return level3
    return level2

fn1 = level1(1)
fn2 = fn1(2)
a = fn2(3)
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(6))));
}

#[test]
fn test_function_as_argument() {
    let source = r#"
def apply(func, x):
    result = func(x)
    return result

def double(x):
    result = x * 2
    return result

fn = double
a = apply(fn, 5)
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(10))));
}

#[test]
fn test_nonlocal_basic() {
    let source = r#"
def outer():
    x = 1
    def inner():
        nonlocal x
        x = x + 1
        return x
    result = inner()
    return result

a = outer()
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(2))));
}

#[test]
fn test_global_basic() {
    let source = r#"
x = 1

def modify_global():
    global x
    x = x + 1
    return x

a = modify_global()
b = x
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
        ("b", StorableValue::Int(BigInt::from(2)))
    );
}

#[test]
fn test_nonlocal_chain() {
    let source = r#"
def level1():
    x = 1
    def level2():
        nonlocal x
        x = x + 1
        def level3():
            nonlocal x
            x = x + 1
            return x
        result = level3()
        return result
    result = level2()
    return result

a = level1()
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(3))));
}

#[test]
fn test_mixed_global_nonlocal() {
    let source = r#"
x = 0
y = 0

def outer():
    y = 1
    def inner():
        global x
        nonlocal y
        x = x + 1
        y = y + 1
        result = y
        return result
    result = inner()
    return result

a = y
b = x
c = y
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
        ("a", StorableValue::Int(BigInt::from(0))),
        ("x", StorableValue::Int(BigInt::from(0))),
        ("y", StorableValue::Int(BigInt::from(0)))
    );
}

#[test]
fn test_function_factory() {
    let source = r#"
def make_multiplier(x):
    def multiply(y):
        result = x * y
        return result
    return multiply

def make_adder(x):
    def add(y):
        result = x + y
        return result
    return add

mult = make_multiplier(3)
add = make_adder(2)
a = mult(4)
b = add(5)
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
        ("a", StorableValue::Int(BigInt::from(12))),
        ("b", StorableValue::Int(BigInt::from(7)))
    );
}

#[test]
fn test_function_composition() {
    let source = r#"
def compose(f, g):
    def composed(x):
        temp = g(x)
        result = f(temp)
        return result
    return composed

def double(x):
    result = x * 2
    return result

def increment(x):
    result = x + 1
    return result

double_fn = double
inc_fn = increment
comp = compose(double_fn, inc_fn)
a = comp(5)  # should be (5+1)*2 = 12
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(12))));
}

#[test]
fn test_recursive_closure() {
    let source = r#"
def make_counter():
    count = 0
    def counter():
        nonlocal count
        count = count + 1
        result = count
        return result
    return counter

counter_fn = make_counter()
a = counter_fn()
b = counter_fn()
c = counter_fn()
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
        ("c", StorableValue::Int(BigInt::from(3)))
    );
}

#[test]
fn test_multiple_nonlocal_scopes() {
    let source = r#"
def level1(x):
    y = x + 1
    def level2():
        nonlocal y
        y = y + 1
        def level3():
            nonlocal y
            y = y + 1
            result = y
            return result
        temp = level3()
        result = temp
        return result
    temp = level2()
    result = temp
    return result

a = level1(1)  # Should be 4 (1 + 1 + 1 + 1)
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(4))));
}

#[test]
fn test_nested_function_shadowing() {
    let source = r#"
def outer(x):
    y = x + 1
    def inner(x):
        z = x + y
        result = z
        return result
    temp = inner(2)
    result = temp
    return result

a = outer(5)  # Should be 8 (2 + (5+1))
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(8))));
}

#[test]
fn test_multiple_global_modifications() {
    let source = r#"
x = 1
y = 2

def modifier1():
    global x, y
    x = x + 1
    y = y + 1
    result = x + y
    return result

def modifier2():
    global x, y
    x = x * 2
    y = y * 2
    result = x * y
    return result

a = modifier1()  # x=2, y=3, returns 5
b = modifier2()  # x=4, y=6, returns 24
c = x  # Should be 4
d = y  # Should be 6
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
        ("b", StorableValue::Int(BigInt::from(24))),
        ("c", StorableValue::Int(BigInt::from(4))),
        ("d", StorableValue::Int(BigInt::from(6)))
    );
}

#[test]
fn test_function_returning_function_chain() {
    let source = r#"
def level1(x):
    def level2(y):
        def level3(z):
            result = x + y + z
            return result
        return level3
    return level2

f1 = level1(1)
f2 = f1(2)
a = f2(3)  # Should be 6 (1+2+3)

g1 = level1(10)
g2 = g1(20)
b = g2(30)  # Should be 60 (10+20+30)
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
        ("a", StorableValue::Int(BigInt::from(6))),
        ("b", StorableValue::Int(BigInt::from(60)))
    );
}

#[test]
fn test_nonlocal_in_returned_function() {
    let source = r#"
def create_counter(start):
    count = start
    def increment():
        nonlocal count
        count = count + 1
        result = count
        return result
    return increment

counter1 = create_counter(0)
a = counter1()  # Should be 1
b = counter1()  # Should be 2

counter2 = create_counter(10)
c = counter2()  # Should be 11
d = counter2()  # Should be 12
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
        ("c", StorableValue::Int(BigInt::from(11))),
        ("d", StorableValue::Int(BigInt::from(12)))
    );
}

#[test]
fn test_global_nonlocal_interaction() {
    let source = r#"
global_var = 1

def outer():
    x = 2
    def middle():
        nonlocal x
        global global_var
        x = x + global_var
        global_var = global_var + 1
        def inner():
            nonlocal x
            global global_var
            result = x + global_var
            return result
        temp = inner()
        result = temp
        return result
    temp = middle()
    result = temp
    return result

a = outer()  # Should compute: x=3 (2+1), global_var=2, then return 5 (3+2)
b = global_var  # Should be 2
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
        ("b", StorableValue::Int(BigInt::from(2)))
    );
}

#[test]
fn test_function_composition_chain() {
    let source = r#"
def create_multiplier(x):
    def multiply(y):
        result = x * y
        return result
    return multiply

def create_adder(x):
    def add(y):
        result = x + y
        return result
    return add

mult2 = create_multiplier(2)
add3 = create_adder(3)
add5 = create_adder(5)

temp1 = mult2(10)  # 20
a = add3(temp1)    # 23
b = add5(a)        # 28
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
        ("a", StorableValue::Int(BigInt::from(23))),
        ("b", StorableValue::Int(BigInt::from(28)))
    );
}

#[test]
fn test_deep_closure_chain() {
    let source = r#"
def level1(a):
    x = a
    def level2(b):
        nonlocal x
        x = x + b
        def level3(c):
            nonlocal x
            x = x + c
            def level4(d):
                nonlocal x
                x = x + d
                result = x
                return result
            return level4
        return level3
    return level2

f1 = level1(1)      # x = 1
f2 = f1(2)          # x = 3
f3 = f2(3)          # x = 6
a = f3(4)           # x = 10

g1 = level1(10)     # new x = 10
g2 = g1(20)         # x = 30
g3 = g2(30)         # x = 60
b = g3(40)          # x = 100
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
        ("a", StorableValue::Int(BigInt::from(10))),
        ("b", StorableValue::Int(BigInt::from(100)))
    );
}

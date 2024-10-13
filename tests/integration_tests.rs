use rustpython_parser::{ast::bigint::BigInt, parse, source_code::LineIndex, Mode};
use simplipy::{
    self,
    datatypes::StorableValue,
    preprocess::preprocess_module,
    state::{init_state, is_fixed_point, tick},
    utils::{env_lookup, lookup},
};

mod common;

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

    lookup_and_assert!(
        state,
        ("z", StorableValue::Int(BigInt::from(5))),
        ("i", StorableValue::Int(BigInt::from(3)))
    );
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

    lookup_and_assert!(state, ("b", StorableValue::Int(BigInt::from(5))),);
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

    lookup_and_assert!(state, ("a", StorableValue::Int(BigInt::from(4))),);
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

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(9))),
        ("b", StorableValue::Int(BigInt::from(9))),
        ("c", StorableValue::Int(BigInt::from(3)))
    );
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

    lookup_and_assert!(
        state,
        ("a", StorableValue::Int(BigInt::from(15))),
        ("b", StorableValue::Int(BigInt::from(9))),
        ("c", StorableValue::Int(BigInt::from(3))),
        ("d", StorableValue::Int(BigInt::from(6)))
    );
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

    lookup_and_assert!(
        state,
        ("x", StorableValue::Int(BigInt::from(4))),
        ("y", StorableValue::Int(BigInt::from(6))),
        ("z", StorableValue::Int(BigInt::from(5))),
    );
}

#[test]
fn test_class_scope() {
    let source = r#"
class B:
    x = 10

class A:
    a = B.x
    B.x = 3
    class B:
        x = 4
        y = B.x
    c = B.x
    d = B.y 

a = A.c
b = A.d
c = A.a

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
        ("a", StorableValue::Int(BigInt::from(4))),
        ("b", StorableValue::Int(BigInt::from(3))),
        ("c", StorableValue::Int(BigInt::from(10))),
    );
}

#[test]
fn test_nested_classes() {
    let source = r#"
class A:
    class B:
        class C:
            x = 3
x = A.B
y = x.C
z = y.x

x.bvar = 10
B = A.B
w = B.bvar

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
        ("z", StorableValue::Int(BigInt::from(3))),
        ("w", StorableValue::Int(BigInt::from(10))),
    );
}

#[test]
fn test_mro() {
    let source = r#"
class A:
    pass

class B(A):
    pass

class C(A):
    pass

class D(B, C):
    pass

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

    let class_d = lookup("D", &state.local_env, &state.global_env, &state.store)
        .and_then(|x| x.as_object().cloned())
        .unwrap();
    let calculated_mro = class_d.metadata.mro.unwrap();
    let expected_mro: Vec<usize> = vec!["D", "B", "C", "A"]
        .iter()
        .map(|x| env_lookup(x, &state.local_env, &state.global_env).unwrap())
        .collect();

    assert_eq!(calculated_mro, expected_mro);
}

#[test]
fn test_complex_mro() {
    let source = r#"
class A:
    pass

class B(A):
    pass

class C(A):
    pass

class D(B, C):
    pass

class E:
    pass

class F(D, E):
    pass

class G(E):
    pass

class H(F, G):
    pass

class I:
    pass

class J(I):
    pass

class Complex(H, J):
    pass

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

    let get_class_idx = |name: &str| env_lookup(name, &state.local_env, &state.global_env).unwrap();

    let test_cases = vec![
        ("A", vec!["A"]),
        ("B", vec!["B", "A"]),
        ("C", vec!["C", "A"]),
        ("D", vec!["D", "B", "C", "A"]),
        ("E", vec!["E"]),
        ("F", vec!["F", "D", "B", "C", "A", "E"]),
        ("G", vec!["G", "E"]),
        ("H", vec!["H", "F", "D", "B", "C", "A", "G", "E"]),
        ("I", vec!["I"]),
        ("J", vec!["J", "I"]),
        (
            "Complex",
            vec!["Complex", "H", "F", "D", "B", "C", "A", "G", "E", "J", "I"],
        ),
    ];

    for (class_name, expected_mro_names) in test_cases {
        let class = lookup(
            class_name,
            &state.local_env,
            &state.global_env,
            &state.store,
        )
        .and_then(|x| x.as_object().cloned())
        .unwrap();
        let calculated_mro = class.metadata.mro.unwrap();
        let expected_mro: Vec<usize> = expected_mro_names
            .into_iter()
            .map(|name| get_class_idx(name))
            .collect();

        assert_eq!(
            calculated_mro, expected_mro,
            "MRO mismatch for class {}",
            class_name
        );
    }
}

#[test]
fn test_mro_usage() {
    let source = r#"
class A:
    def method1(self):
        return "A.method1"
    
    def method2(self):
        return "A.method2"

class B(A):
    def method1(self):
        return "B.method1"
    
    def method3(self):
        return "B.method3"

class C(A):
    def method2(self):
        return "C.method2"

class D(B, C):
    def method3(self):
        return "D.method3"

class E:
    def method1(self):
        return "E.method1"
    
    def method4(self):
        return "E.method4"

class F(D, E):
    def method4(self):
        return "F.method4"

class G:
    def method5(self):
        return "G.method5"

class Complex(F, G):
    def __init__(self):
        return self

    def method5(self):
        return "Complex.method5"

obj = Complex()

result1 = obj.method1()
result2 = obj.method2()
result3 = obj.method3()
result4 = obj.method4()
result5 = obj.method5()

expected1 = "B.method1"
expected2 = "C.method2"
expected3 = "D.method3"
expected4 = "F.method4"
expected5 = "Complex.method5"


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
        ("result1", StorableValue::String(String::from("B.method1"))),
        ("result2", StorableValue::String(String::from("C.method2"))),
        ("result3", StorableValue::String(String::from("D.method3"))),
        ("result4", StorableValue::String(String::from("F.method4"))),
        (
            "result5",
            StorableValue::String(String::from("Complex.method5"))
        ),
    );
}

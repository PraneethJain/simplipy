use std::collections::BTreeSet;

use rustpython_parser::ast::{self, Expr, Identifier};

use crate::datatypes::{Env, Object, StorableValue, Store};

pub fn env_lookup(var: &str, local_env: &Option<Env>, global_env: &Env) -> Option<usize> {
    if let Some(local_env) = local_env {
        if let Some(&idx) = local_env.get(var) {
            return Some(idx);
        }
    }
    global_env.get(var).cloned()
}

pub fn lookup<'a>(
    var: &str,
    local_env: &Option<Env>,
    global_env: &Env,
    store: &'a Store,
) -> Option<&'a StorableValue> {
    env_lookup(var, local_env, global_env).and_then(|idx| store.get(idx))
}

pub fn class_env_lookup<'a>(
    attr: &str,
    class_env: &Env,
    store: &'a Store,
) -> Option<&'a StorableValue> {
    let idx = *class_env.get(attr)?;
    store.get(idx)

    // lookup through mro here when doing inheritance
}

pub fn obj_lookup<'a>(
    obj_var: &str,
    attr: &str,
    local_env: &Option<Env>,
    global_env: &Env,
    store: &'a Store,
) -> Option<StorableValue> {
    let obj = lookup(obj_var, local_env, global_env, store)?
        .as_object()
        .expect("Object must be stored as object type");
    let obj_idx = env_lookup(obj_var, local_env, global_env)?;
    let obj_env = store
        .get(obj.env_addr)
        .and_then(|x| x.as_env().cloned())
        .expect("Object must have its environment initialized");
    if let Some(val) = lookup(&attr, &None, &obj_env, store).cloned() {
        Some(val)
    } else if let Some(class_env_addr) = obj.class {
        let class_env = store
            .get(class_env_addr)
            .and_then(|x| x.as_env())
            .expect("Class environment must be initialized");
        let res = class_env_lookup(attr, class_env, store).cloned();

        if let Some(StorableValue::DefinitionClosure(func_lineno, func_env, mut formals)) = res {
            // return a bound method
            let mut func_env = func_env.unwrap_or_default();
            let self_var = formals.remove(0);
            func_env.insert(self_var, obj_idx);
            Some(StorableValue::DefinitionClosure(
                func_lineno,
                Some(func_env),
                formals,
            ))
        } else {
            res
        }
    } else {
        None
    }
}

pub fn eval(
    expr: &Expr,
    local_env: &Option<Env>,
    global_env: &Env,
    store: &Store,
) -> Option<StorableValue> {
    match expr {
        Expr::Name(ast::ExprName { id, .. }) => {
            lookup(id.as_str(), local_env, global_env, store).cloned()
        }
        Expr::Constant(ast::ExprConstant { value, .. }) => {
            use ast::Constant;
            match value {
                Constant::None => Some(StorableValue::None),
                Constant::Bool(bool_val) => Some(StorableValue::Bool(*bool_val)),
                Constant::Str(string_val) => Some(StorableValue::String(string_val.to_string())),
                Constant::Int(int_val) => Some(StorableValue::Int(int_val.clone())),
                Constant::Float(float_val) => Some(StorableValue::Float(*float_val)),
                Constant::Bytes(_) => todo!(),
                Constant::Ellipsis => todo!(),
                Constant::Tuple(_) => todo!(),
                Constant::Complex { .. } => todo!(),
            }
        }
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            let obj_var = value
                .as_name_expr()
                .expect("Object fields must be accessed directly")
                .id
                .as_str();
            obj_lookup(obj_var, attr, local_env, global_env, store)
        }
        Expr::UnaryOp(ast::ExprUnaryOp { op, operand, .. }) => {
            let val = eval(operand, local_env, global_env, store)?;
            use ast::UnaryOp;
            match op {
                UnaryOp::USub => -val,
                UnaryOp::Not | UnaryOp::UAdd | UnaryOp::Invert => todo!(),
            }
        }
        Expr::BinOp(ast::ExprBinOp {
            left, op, right, ..
        }) => {
            let left_val = eval(left, local_env, global_env, store)?;
            let right_val = eval(right, local_env, global_env, store)?;

            use ast::Operator;
            match op {
                Operator::Add => left_val + right_val,
                Operator::Sub => left_val - right_val,
                Operator::Mult => left_val * right_val,
                Operator::FloorDiv => left_val.floordiv(right_val),
                Operator::Div => left_val / right_val,
                Operator::Mod => left_val % right_val,
                Operator::Pow
                | Operator::LShift
                | Operator::RShift
                | Operator::BitOr
                | Operator::BitXor
                | Operator::BitAnd
                | Operator::MatMult => todo!(),
            }
        }
        Expr::Compare(ast::ExprCompare {
            left,
            ops,
            comparators,
            ..
        }) => {
            let mut left_val = &eval(left, local_env, global_env, store)?;
            let right_vals = comparators
                .iter()
                .map(|x| eval(x, local_env, global_env, store))
                .collect::<Option<Vec<_>>>()?;
            let mut result = true;
            for (right_val, op) in right_vals.iter().zip(ops.iter()) {
                use ast::CmpOp;
                result &= match op {
                    CmpOp::Eq => left_val == right_val,
                    CmpOp::NotEq => left_val != right_val,
                    CmpOp::Lt => left_val < right_val,
                    CmpOp::LtE => left_val <= right_val,
                    CmpOp::Gt => left_val > right_val,
                    CmpOp::GtE => left_val >= right_val,
                    CmpOp::Is | CmpOp::IsNot | CmpOp::In | CmpOp::NotIn => todo!(),
                };
                left_val = right_val;
            }

            Some(StorableValue::Bool(result))
        }
        Expr::BoolOp(ast::ExprBoolOp { op, values, .. }) => {
            let vals = values
                .iter()
                .map(|x| eval(x, local_env, global_env, store).and_then(|x| x.bool()))
                .collect::<Option<Vec<_>>>()?;
            Some(StorableValue::Bool(match op {
                ast::BoolOp::And => vals.iter().all(|&x| x),
                ast::BoolOp::Or => vals.iter().any(|&x| x),
            }))
        }
        Expr::NamedExpr(_) => todo!(),

        Expr::Lambda(_) => todo!(),
        Expr::IfExp(_) => todo!(),
        Expr::Dict(_) => todo!(),
        Expr::Set(_) => todo!(),
        Expr::ListComp(_) => todo!(),
        Expr::SetComp(_) => todo!(),
        Expr::DictComp(_) => todo!(),
        Expr::GeneratorExp(_) => todo!(),
        Expr::Await(_) => todo!(),
        Expr::Yield(_) => todo!(),
        Expr::YieldFrom(_) => todo!(),
        Expr::Call(_) => todo!(),
        Expr::FormattedValue(_) => todo!(),
        Expr::JoinedStr(_) => todo!(),
        Expr::Subscript(_) => todo!(),
        Expr::Starred(_) => todo!(),
        Expr::List(_) => todo!(),
        Expr::Tuple(_) => todo!(),
        Expr::Slice(_) => todo!(),
    }
}

pub fn update(
    var: &str,
    val: StorableValue,
    local_env: &Option<Env>,
    global_env: &Env,
    mut store: Store,
) -> Option<Store> {
    let store_idx = env_lookup(var, local_env, global_env)?;
    let store_val = store.get_mut(store_idx)?;
    *store_val = val;

    Some(store)
}

pub fn update_obj(
    var: String,
    val: StorableValue,
    obj: &Object,
    mut store: Store,
) -> Option<Store> {
    let mut obj_env = store.get(obj.env_addr)?.as_env().cloned()?;
    obj_env
        .entry(var)
        .and_modify(|&mut x| store[x] = val.clone())
        .or_insert_with(|| {
            store.push(val);
            store.len() - 1
        });
    store[obj.env_addr] = StorableValue::Env(obj_env);

    Some(store)
}

pub fn update_class_env(
    name: &Identifier,
    val: StorableValue,
    class_env: &mut Env,
    mut store: Store,
) -> Store {
    class_env
        .entry(name.to_string())
        .and_modify(|idx| {
            store[*idx] = val.clone();
        })
        .or_insert_with(|| {
            store.push(val);
            store.len() - 1
        });

    store
}

pub fn assign_in_class_context(
    var: &Expr,
    value: &Expr,
    local_env: &Option<Env>,
    global_env: &Env,
    class_env: &mut Env,
    store: Store,
) -> Option<Store> {
    let lookup_env = {
        if let Some(local_env) = local_env {
            let mut env = local_env.clone();
            env.extend(class_env.clone());
            env
        } else {
            class_env.clone()
        }
    };
    let val = eval(value, &Some(lookup_env.clone()), global_env, &store)?;

    assign_val_in_class_context(var, val, &Some(lookup_env), global_env, class_env, store)
}

pub fn assign_val_in_class_context(
    var: &Expr,
    val: StorableValue,
    lookup_env: &Option<Env>,
    global_env: &Env,
    class_env: &mut Env,
    mut store: Store,
) -> Option<Store> {
    match var {
        ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            let obj = lookup(
                value.as_name_expr().unwrap().id.as_str(),
                &lookup_env,
                global_env,
                &store,
            )?
            .as_object()
            .unwrap()
            .clone();
            store = update_obj(attr.to_string(), val, &obj, store)?;
        }
        ast::Expr::Name(name) => store = update_class_env(&name.id, val, class_env, store),
        _ => unimplemented!(),
    }

    Some(store)
}

pub fn assign_in_lexical_context(
    var: &Expr,
    value: &Expr,
    local_env: &Option<Env>,
    global_env: &Env,
    store: Store,
) -> Option<Store> {
    let val = eval(value, local_env, global_env, &store)?;
    assign_val_in_lexical_context(var, val, local_env, global_env, store)
}

pub fn assign_val_in_lexical_context(
    var: &Expr,
    val: StorableValue,
    local_env: &Option<Env>,
    global_env: &Env,
    mut store: Store,
) -> Option<Store> {
    match var {
        ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            let obj = lookup(
                value.as_name_expr().unwrap().id.as_str(),
                local_env,
                global_env,
                &store,
            )?
            .as_object()
            .unwrap()
            .clone();
            store = update_obj(attr.to_string(), val, &obj, store)?;
        }
        ast::Expr::Name(name) => {
            store = update(&name.id, val, local_env, global_env, store)?;
        }
        _ => unimplemented!(),
    }

    Some(store)
}

pub fn update_in_global_env(
    var: String,
    val: StorableValue,
    mut global_env: Env,
    mut store: Store,
) -> (Env, Store) {
    global_env
        .entry(var)
        .and_modify(|x| store[*x] = val.clone())
        .or_insert_with(|| {
            store.push(val);
            store.len() - 1
        });

    (global_env, store)
}

pub fn setup_func_call(
    func_env: Option<Env>,
    mut global_env: Env,
    mut store: Store,
    decvars: &BTreeSet<&str>,
    globals: &BTreeSet<&str>,
    formals: Vec<String>,
    vals: Vec<StorableValue>,
) -> Option<(Env, Env, Store)> {
    let n = store.len();
    let mut func_env = func_env.unwrap_or_default();
    func_env.extend(
        decvars
            .iter()
            .filter(|&x| !globals.contains(x))
            .enumerate()
            .map(|(i, x)| (x.to_string(), n + i)),
    );
    store.extend(vec![StorableValue::Bottom; func_env.len()]);

    for var in globals {
        let var_str = var.to_string();
        if !global_env.contains_key(&var_str) {
            (global_env, store) =
                update_in_global_env(var_str, StorableValue::Bottom, global_env, store);
        }
    }

    for (formal, val) in formals.into_iter().zip(vals.into_iter()) {
        store = update(&formal, val, &Some(func_env.clone()), &global_env, store)?;
    }

    Some((func_env, global_env, store))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::datatypes::Env;
    use rustpython_parser::{ast::bigint::BigInt, parse, Mode};
    use std::collections::BTreeMap;

    fn eval_from_src(
        source: &str,
        local_env: &Option<Env>,
        global_env: &Env,
        store: &Store,
    ) -> Option<StorableValue> {
        let ast = parse(source, Mode::Expression, "<embedded>").unwrap();
        let expr = &ast.as_expression().unwrap().body;
        eval(expr, local_env, global_env, store)
    }

    #[test]
    fn eval_simple() {
        let source = r#"1 + 2 * 3 + 2"#;
        let result = eval_from_src(source, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Int(BigInt::from(9)));
    }

    #[test]
    fn eval_substitution() {
        let source = r#"x + y*y*y + z + 2*8 + 8/4"#;
        let result = eval_from_src(
            source,
            &Some(BTreeMap::from([
                ("x".to_string(), 0),
                ("y".to_string(), 1),
                ("z".to_string(), 2),
            ])),
            &BTreeMap::new(),
            &vec![
                StorableValue::Int(BigInt::from(0)),
                StorableValue::Int(BigInt::from(2)),
                StorableValue::Int(BigInt::from(10)),
            ],
        );
        assert_eq!(result.unwrap(), StorableValue::Int(BigInt::from(36)));
    }

    #[test]
    fn eval_string() {
        let source = r#"x +  "hello""#;
        let result = eval_from_src(
            source,
            &Some(BTreeMap::from([("x".to_string(), 0)])),
            &BTreeMap::new(),
            &vec![StorableValue::String(String::from("world "))],
        );
        assert_eq!(
            result.unwrap(),
            StorableValue::String(String::from("world hello"))
        );
    }

    #[test]
    fn eval_conditions() {
        let result = eval_from_src(r#"1 < 2 < 3 < 4 < 5"#, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(r#"1 < 2 < 3 < 4 < 2"#, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Bool(false));

        let result = eval_from_src(r#"1 < 5 and 3 > 2"#, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(r#"1 < 2 < 4 or 2 > 4"#, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(r#"1 >= 4 or 4 <= 1"#, &None, &BTreeMap::new(), &vec![]);
        assert_eq!(result.unwrap(), StorableValue::Bool(false));

        let result = eval_from_src(
            r#"1 >= 4 or 4 <= 1 or True"#,
            &None,
            &BTreeMap::new(),
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(true));
    }
}

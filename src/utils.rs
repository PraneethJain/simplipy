use rustpython_parser::ast::{self, Expr};

use crate::datatypes::{Env, StorableValue, Store};

pub fn env_lookup(var: &str, env: &Env) -> Option<usize> {
    env.iter()
        .rev()
        .find_map(|local_env| local_env.mapping.get(var).copied())
}

pub fn lookup<'a>(var: &str, env: &Env, store: &'a Store) -> Option<&'a StorableValue> {
    env_lookup(var, env).and_then(|idx| store.get(idx))
}

pub fn eval(expr: &Expr, env: &Env, store: &Store) -> Option<StorableValue> {
    match expr {
        Expr::Name(ast::ExprName { id, .. }) => lookup(id.as_str(), env, store).cloned(),
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
            let obj = lookup(obj_var, env, store)?
                .clone()
                .as_object()
                .expect("Object must be stored as object type");
            let obj_env = store
                .get(obj.flat_env_addr)
                .and_then(|x| x.clone().as_flat_env())
                .expect("Object must have its environment initialized");
            lookup(attr, &vec![obj_env], store).cloned()
        }
        Expr::BinOp(ast::ExprBinOp {
            left, op, right, ..
        }) => {
            let left_val = eval(left, env, store)?;
            let right_val = eval(right, env, store)?;

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
            let mut left_val = &eval(left, env, store)?;
            let right_vals = comparators
                .iter()
                .map(|x| eval(x, env, store))
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
                .map(|x| eval(x, env, store).and_then(|x| x.as_bool()))
                .collect::<Option<Vec<_>>>()?;
            Some(StorableValue::Bool(match op {
                ast::BoolOp::And => vals.iter().all(|&x| x),
                ast::BoolOp::Or => vals.iter().any(|&x| x),
            }))
        }
        Expr::NamedExpr(_) => todo!(),
        Expr::UnaryOp(_) => todo!(),
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

pub fn update(var: &str, val: StorableValue, env: &Env, mut store: Store) -> Option<Store> {
    let store_idx = env_lookup(var, env)?;
    let store_val = store.get_mut(store_idx)?;
    *store_val = val;

    Some(store)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::datatypes::FlatEnv;
    use rustpython_parser::{ast::bigint::BigInt, parse, Mode};
    use std::collections::BTreeMap;

    fn eval_from_src(source: &str, env: &Env, store: &Store) -> Option<StorableValue> {
        let ast = parse(source, Mode::Expression, "<embedded>").unwrap();
        let expr = &ast.as_expression().unwrap().body;
        eval(expr, env, store)
    }

    #[test]
    fn eval_simple() {
        let source = r#"1 + 2 * 3 + 2"#;
        let result = eval_from_src(
            source,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Int(BigInt::from(9)));
    }

    #[test]
    fn eval_substitution() {
        let source = r#"x + y*y*y + z + 2*8 + 8/4"#;
        let result = eval_from_src(
            source,
            &vec![FlatEnv::new(
                BTreeMap::from([
                    ("x".to_string(), 0),
                    ("y".to_string(), 1),
                    ("z".to_string(), 2),
                ]),
                "".to_string(),
            )],
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
            &vec![FlatEnv::new(
                BTreeMap::from([("x".to_string(), 0)]),
                "".to_string(),
            )],
            &vec![StorableValue::String(String::from("world "))],
        );
        assert_eq!(
            result.unwrap(),
            StorableValue::String(String::from("world hello"))
        );
    }

    #[test]
    fn eval_conditions() {
        let result = eval_from_src(
            r#"1 < 2 < 3 < 4 < 5"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(
            r#"1 < 2 < 3 < 4 < 2"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(false));

        let result = eval_from_src(
            r#"1 < 5 and 3 > 2"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(
            r#"1 < 2 < 4 or 2 > 4"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(true));

        let result = eval_from_src(
            r#"1 >= 4 or 4 <= 1"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(false));

        let result = eval_from_src(
            r#"1 >= 4 or 4 <= 1 or True"#,
            &vec![FlatEnv::new(BTreeMap::new(), "".to_string())],
            &vec![],
        );
        assert_eq!(result.unwrap(), StorableValue::Bool(true));
    }
}

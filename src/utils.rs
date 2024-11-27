use std::collections::{BTreeMap, BTreeSet};

use rustpython_parser::ast::{self, Expr};

use crate::datatypes::{Env, EnvId, Envs, Parent, StorableValue};

pub fn find_lexical_block(
    lineno: usize,
    blocks: &BTreeMap<usize, (usize, usize)>,
) -> Option<usize> {
    let mut size = usize::MAX;
    let mut res = None;
    for (block_lineno, range) in blocks.iter() {
        if lineno >= range.0 && lineno <= range.1 && (res == None || range.1 - range.0 < size) {
            res = Some(*block_lineno);
            size = range.1 - range.0;
        }
    }
    res
}

pub fn lookup(
    var: &str,
    env_id: EnvId,
    envs: &Envs,
    parent: &Parent,
    globals: &BTreeSet<&str>,
) -> Option<StorableValue> {
    if globals.contains(var) {
        return envs[&0].get(var).cloned();
    }

    if envs[&env_id].contains_key(var) {
        let res = envs[&env_id][var].clone();
        if res != StorableValue::Bottom {
            Some(res)
        } else {
            None
        }
    } else if env_id == 0 {
        None
    } else {
        lookup(var, parent[&env_id], envs, parent, globals)
    }
}

pub fn eval(
    expr: &Expr,
    env_id: EnvId,
    envs: &Envs,
    parent: &Parent,
    globals: &BTreeSet<&str>,
) -> Option<StorableValue> {
    match expr {
        Expr::Name(ast::ExprName { id, .. }) => lookup(id.as_str(), env_id, envs, parent, globals),
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
        Expr::Attribute(_) => todo!(),
        Expr::UnaryOp(ast::ExprUnaryOp { op, operand, .. }) => {
            let val = eval(operand, env_id, envs, parent, globals)?;
            use ast::UnaryOp;
            match op {
                UnaryOp::USub => -val,
                UnaryOp::Not => !val,
                UnaryOp::UAdd | UnaryOp::Invert => todo!(),
            }
        }
        Expr::BinOp(ast::ExprBinOp {
            left, op, right, ..
        }) => {
            let left_val = eval(left, env_id, envs, parent, globals)?;
            let right_val = eval(right, env_id, envs, parent, globals)?;

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
            let mut left_val = &eval(left, env_id, envs, parent, globals)?;
            let right_vals = comparators
                .iter()
                .map(|x| eval(x, env_id, envs, parent, globals))
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
                .map(|x| eval(x, env_id, envs, parent, globals).and_then(|x| x.bool()))
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
    env_id: EnvId,
    mut envs: Envs,
    parent: &Parent,
) -> Envs {
    if envs[&env_id].contains_key(var) || env_id == 0 {
        envs.get_mut(&env_id).unwrap().insert(var.to_string(), val);
        envs
    } else {
        update(var, val, parent[&env_id], envs, parent)
    }
}

pub fn setup_func_call(
    mut envs: Envs,
    decvars: &BTreeSet<&str>,
    formals: Vec<String>,
    vals: Vec<StorableValue>,
) -> Envs {
    let mut func_env: Env = decvars
        .iter()
        .map(|x| (x.to_string(), StorableValue::Bottom))
        .collect();

    for (formal, val) in formals.into_iter().zip(vals.into_iter()) {
        func_env.insert(formal, val);
    }

    envs.insert(envs.len(), func_env);

    envs
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::datatypes::Env;
//     use rustpython_parser::{ast::bigint::BigInt, parse, Mode};
//     use std::collections::BTreeMap;

//     fn eval_from_src(
//         source: &str,
//         local_env: &Option<Env>,
//         global_env: &Env,
//         store: &Store,
//     ) -> Option<StorableValue> {
//         let ast = parse(source, Mode::Expression, "<embedded>").unwrap();
//         let expr = &ast.as_expression().unwrap().body;
//         eval(expr, local_env, global_env, store)
//     }

//     #[test]
//     fn eval_simple() {
//         let source = r#"1 + 2 * 3 + 2"#;
//         let result = eval_from_src(source, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Int(BigInt::from(9)));
//     }

//     #[test]
//     fn eval_substitution() {
//         let source = r#"x + y*y*y + z + 2*8 + 8/4"#;
//         let result = eval_from_src(
//             source,
//             &Some(BTreeMap::from([
//                 ("x".to_string(), 0),
//                 ("y".to_string(), 1),
//                 ("z".to_string(), 2),
//             ])),
//             &BTreeMap::new(),
//             &vec![
//                 StorableValue::Int(BigInt::from(0)),
//                 StorableValue::Int(BigInt::from(2)),
//                 StorableValue::Int(BigInt::from(10)),
//             ],
//         );
//         assert_eq!(result.unwrap(), StorableValue::Int(BigInt::from(36)));
//     }

//     #[test]
//     fn eval_string() {
//         let source = r#"x +  "hello""#;
//         let result = eval_from_src(
//             source,
//             &Some(BTreeMap::from([("x".to_string(), 0)])),
//             &BTreeMap::new(),
//             &vec![StorableValue::String(String::from("world "))],
//         );
//         assert_eq!(
//             result.unwrap(),
//             StorableValue::String(String::from("world hello"))
//         );
//     }

//     #[test]
//     fn eval_conditions() {
//         let result = eval_from_src(r#"1 < 2 < 3 < 4 < 5"#, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Bool(true));

//         let result = eval_from_src(r#"1 < 2 < 3 < 4 < 2"#, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Bool(false));

//         let result = eval_from_src(r#"1 < 5 and 3 > 2"#, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Bool(true));

//         let result = eval_from_src(r#"1 < 2 < 4 or 2 > 4"#, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Bool(true));

//         let result = eval_from_src(r#"1 >= 4 or 4 <= 1"#, &None, &BTreeMap::new(), &vec![]);
//         assert_eq!(result.unwrap(), StorableValue::Bool(false));

//         let result = eval_from_src(
//             r#"1 >= 4 or 4 <= 1 or True"#,
//             &None,
//             &BTreeMap::new(),
//             &vec![],
//         );
//         assert_eq!(result.unwrap(), StorableValue::Bool(true));
//     }
// }

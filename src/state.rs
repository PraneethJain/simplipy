use std::collections::BTreeMap;

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{
    ApplicationClosure, DefinitionClosure, FlatEnv, Object, State, StorableValue,
};
use crate::preprocess::Static;
use crate::utils::{eval, lookup, update, update_obj};

pub fn init_state(static_info: &Static) -> State {
    State {
        lineno: *static_info
            .statements
            .keys()
            .min()
            .expect("Atleast one statement should be present"),
        env: vec![FlatEnv::new(
            static_info.decvars[&0]
                .iter()
                .enumerate()
                .map(|(a, b)| (b.to_string(), a))
                .collect(),
            "Global".to_string(),
        )],
        stack: vec![],
        store: vec![StorableValue::Bottom; static_info.decvars[&0].len()],
        class_envs: vec![],
    }
}

pub fn tick(mut state: State, static_info: &Static) -> Option<State> {
    let lineno = state.lineno;
    let stmt = static_info.statements[&lineno];

    let mut next_state = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            if let Some(ast::ExprCall { func, args, .. }) = value.as_call_expr() {
                let func_name = func.as_name_expr()?.id.as_str();
                match lookup(func_name, &state.env, &state.store)?.clone() {
                    StorableValue::DefinitionClosure(DefinitionClosure(func_lineno, func_env)) => {
                        let func_stmt =
                            static_info.statements[&func_lineno].as_function_def_stmt()?;
                        let formals = func_stmt
                            .args
                            .args
                            .iter()
                            .map(|x| x.def.arg.as_str())
                            .collect::<Vec<_>>();

                        if func_stmt.args.args.len() != args.len() {
                            panic!("Function call with wrong number of arguments");
                        }

                        let vals = args
                            .iter()
                            .map(|x| eval(x, &state.env, &state.store))
                            .collect::<Option<Vec<_>>>()?;

                        let return_closure =
                            ApplicationClosure(lineno, state.env, state.class_envs);
                        state.class_envs = vec![];
                        state.stack.push(return_closure);

                        let n = state.store.len();
                        state.env = func_env;
                        state.env.push(FlatEnv::new(
                            static_info.decvars[&func_lineno]
                                .iter()
                                .enumerate()
                                .map(|(i, x)| (x.to_string(), n + i))
                                .collect(),
                            func_name.to_string(),
                        ));
                        state.store.extend(vec![
                            StorableValue::Bottom;
                            static_info.decvars[&func_lineno].len()
                        ]);

                        for (formal, val) in formals.into_iter().zip(vals.into_iter()) {
                            state.store = update(formal, val, &state.env, state.store)?;
                        }

                        State {
                            lineno: static_info.block[&func_lineno].0,
                            ..state
                        }
                    }
                    StorableValue::Object(Object {
                        class: None,
                        flat_env_addr,
                    }) => {
                        let class_env = state
                            .store
                            .get(flat_env_addr)
                            .and_then(|x| x.as_flat_env().cloned())
                            .expect("Object must have an initialized flat environment");

                        let DefinitionClosure(func_lineno, func_env) =
                            lookup("__init__", &vec![class_env], &state.store)
                                .and_then(|x| x.clone().closure())
                                .expect("Class must have a __init__ function");

                        let func_stmt =
                            static_info.statements[&func_lineno].as_function_def_stmt()?;

                        let formals = func_stmt
                            .args
                            .args
                            .iter()
                            .map(|x| x.def.arg.as_str())
                            .collect::<Vec<_>>();

                        if func_stmt.args.args.len() != args.len() + 1 {
                            panic!("Function call with wrong number of arguments");
                        }

                        let mut vals = args
                            .iter()
                            .map(|x| eval(x, &state.env, &state.store))
                            .collect::<Option<Vec<_>>>()?;

                        let obj_env = FlatEnv::new(BTreeMap::new(), "".to_string());
                        state.store.push(StorableValue::FlatEnv(obj_env));
                        let obj = Object {
                            class: Some(flat_env_addr),
                            flat_env_addr: state.store.len() - 1,
                        };
                        vals.insert(0, StorableValue::Object(obj));

                        let return_closure =
                            ApplicationClosure(lineno, state.env, state.class_envs);
                        state.class_envs = vec![];
                        state.stack.push(return_closure);

                        let n = state.store.len();
                        state.env = func_env;
                        state.env.push(FlatEnv::new(
                            static_info.decvars[&func_lineno]
                                .iter()
                                .enumerate()
                                .map(|(i, x)| (x.to_string(), n + i))
                                .collect(),
                            func_name.to_string(),
                        ));
                        state.store.extend(vec![
                            StorableValue::Bottom;
                            static_info.decvars[&func_lineno].len()
                        ]);

                        for (formal, val) in formals.into_iter().zip(vals.into_iter()) {
                            state.store = update(formal, val, &state.env, state.store)?;
                        }

                        State {
                            lineno: static_info.block[&func_lineno].0,
                            ..state
                        }
                    }
                    _ => panic!("Expected callable"),
                }
            } else {
                if let Some((_, class_env)) = state.class_envs.last_mut() {
                    // In some class
                    let mut lookup_env = state.env.clone();
                    lookup_env.push(class_env.clone());
                    let val = eval(value, &lookup_env, &state.store)?;

                    match &targets[0] {
                        ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                            let obj = lookup(
                                value.as_name_expr().unwrap().id.as_str(),
                                &lookup_env,
                                &state.store,
                            )?
                            .as_object()
                            .unwrap()
                            .clone();
                            state.store = update_obj(attr.to_string(), val, &obj, state.store)?;
                        }
                        ast::Expr::Name(name) => {
                            class_env
                                .mapping
                                .entry(name.id.to_string())
                                .and_modify(|idx| {
                                    state.store[*idx] = val.clone();
                                })
                                .or_insert_with(|| {
                                    state.store.push(val);
                                    state.store.len() - 1
                                });
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    // Not in a class
                    let val = eval(value, &state.env, &state.store)?;
                    match &targets[0] {
                        ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                            let obj = lookup(
                                value.as_name_expr().unwrap().id.as_str(),
                                &state.env,
                                &state.store,
                            )?
                            .as_object()
                            .unwrap()
                            .clone();
                            state.store = update_obj(attr.to_string(), val, &obj, state.store)?;
                        }
                        ast::Expr::Name(name) => {
                            state.store = update(&name.id, val, &state.env, state.store)?;
                        }
                        _ => unimplemented!(),
                    }
                }

                State {
                    lineno: static_info.next_stmt[&lineno],
                    ..state
                }
            }
        }
        Stmt::While(ast::StmtWhile { test, .. }) | Stmt::If(ast::StmtIf { test, .. }) => {
            let res = eval(&test, &state.env, &state.store)?;
            let bool_res = res.bool()?;
            State {
                lineno: if bool_res {
                    static_info.true_stmt[&lineno]
                } else {
                    static_info.false_stmt[&lineno]
                },
                ..state
            }
        }
        Stmt::Continue(ast::StmtContinue { .. })
        | Stmt::Break(ast::StmtBreak { .. })
        | Stmt::Pass(ast::StmtPass { .. }) => State {
            lineno: static_info.next_stmt[&lineno],
            ..state
        },
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => {
            let closure =
                StorableValue::DefinitionClosure(DefinitionClosure(lineno, state.env.clone()));

            if let Some((_, class_env)) = state.class_envs.last_mut() {
                class_env
                    .mapping
                    .entry(name.to_string())
                    .and_modify(|idx| {
                        state.store[*idx] = closure.clone();
                    })
                    .or_insert_with(|| {
                        state.store.push(closure);
                        state.store.len() - 1
                    });
            } else {
                state.store = update(name, closure, &state.env, state.store)?;
            }

            State {
                lineno: static_info.next_stmt[&lineno],
                ..state
            }
        }
        Stmt::Return(ast::StmtReturn { value, .. }) => {
            let val = if let Some(expr) = value {
                eval(expr, &state.env, &state.store)?
            } else {
                StorableValue::None
            };

            let ApplicationClosure(ret_lineno, ret_env, ret_class_envs) = state
                .stack
                .pop()
                .expect("Non empty stack during function return");

            let targets = static_info.statements[&ret_lineno]
                .as_assign_stmt()
                .expect("Functions must be called in assignment statements")
                .targets
                .clone();

            if let Some((_, class_env)) = state.class_envs.last_mut() {
                // In some class

                match &targets[0] {
                    ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                        let mut lookup_env = state.env.clone();
                        lookup_env.push(class_env.clone());
                        let obj = lookup(
                            value.as_name_expr().unwrap().id.as_str(),
                            &lookup_env,
                            &state.store,
                        )?
                        .as_object()
                        .unwrap()
                        .clone();
                        state.store = update_obj(attr.to_string(), val, &obj, state.store)?;
                    }
                    ast::Expr::Name(name) => {
                        class_env
                            .mapping
                            .entry(name.id.to_string())
                            .and_modify(|idx| {
                                state.store[*idx] = val.clone();
                            })
                            .or_insert_with(|| {
                                state.store.push(val);
                                state.store.len() - 1
                            });
                    }
                    _ => unimplemented!(),
                }
            } else {
                // Not in a class
                match &targets[0] {
                    ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                        let obj = lookup(
                            value.as_name_expr().unwrap().id.as_str(),
                            &state.env,
                            &state.store,
                        )?
                        .as_object()
                        .unwrap()
                        .clone();
                        state.store = update_obj(attr.to_string(), val, &obj, state.store)?;
                    }
                    ast::Expr::Name(name) => {
                        state.store = update(&name.id, val, &ret_env, state.store)?;
                    }
                    _ => unimplemented!(),
                }
            }

            State {
                lineno: static_info.next_stmt[&ret_lineno],
                env: ret_env,
                class_envs: ret_class_envs,
                ..state
            }
        }
        Stmt::ClassDef(ast::StmtClassDef { name, bases, .. }) => {
            state
                .class_envs
                .push((lineno, FlatEnv::new(BTreeMap::new(), name.to_string())));
            State {
                lineno: static_info.block[&lineno].0,
                ..state
            }
        }
        Stmt::Expr(_) => todo!(),
        Stmt::Global(_) => todo!(),
        Stmt::Nonlocal(_) => todo!(),
        Stmt::Import(_) => todo!(),
        Stmt::ImportFrom(_) => todo!(),
        Stmt::Try(_) => todo!(),
        Stmt::TryStar(_) => todo!(),
        Stmt::Raise(_) => todo!(),
        Stmt::AugAssign(_) => unimplemented!(),
        Stmt::For(_) => unimplemented!(),
        Stmt::AsyncFunctionDef(_) => unimplemented!(),
        Stmt::AnnAssign(_) => unimplemented!(),
        Stmt::Assert(_) => unimplemented!(),
        Stmt::With(_) => unimplemented!(),
        Stmt::AsyncWith(_) => unimplemented!(),
        Stmt::AsyncFor(_) => unimplemented!(),
        Stmt::Match(_) => unimplemented!(),
        Stmt::TypeAlias(_) => unimplemented!(),
        Stmt::Delete(_) => unimplemented!(),
    };

    while let Some((class_lineno, _)) = next_state.class_envs.last().cloned() {
        let class_end_lineno = static_info.block[&class_lineno].1;
        if next_state.lineno == static_info.next_stmt[&class_end_lineno] {
            let (_, class_env) = next_state.class_envs.pop().unwrap();
            next_state.store.push(StorableValue::FlatEnv(class_env));
            let flat_env_addr = next_state.store.len() - 1;
            let class_name = static_info.statements[&class_lineno]
                .as_class_def_stmt()
                .unwrap()
                .name
                .clone();
            let class_object = StorableValue::Object(Object {
                class: None,
                flat_env_addr,
            });

            if let Some((_, class_env)) = next_state.class_envs.last_mut() {
                class_env
                    .mapping
                    .entry(class_name.to_string())
                    .and_modify(|idx| {
                        next_state.store[*idx] = class_object.clone();
                    })
                    .or_insert_with(|| {
                        next_state.store.push(class_object);
                        next_state.store.len() - 1
                    });
            } else {
                next_state.store = update(
                    class_name.as_str(),
                    class_object,
                    &next_state.env,
                    next_state.store,
                )?;
            }
        } else {
            break;
        }
    }

    Some(State { ..next_state })
}

pub fn is_fixed_point(state: &State, static_info: &Static) -> bool {
    let lineno = state.lineno;
    static_info.next_stmt[&lineno] == lineno
}

use std::collections::BTreeMap;

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{Context, Object, ObjectMetadata, State, StorableValue};
use crate::preprocess::Static;
use crate::utils::{
    assign_in_class_context, assign_in_lexical_context, assign_val_in_class_context,
    assign_val_in_lexical_context, env_lookup, eval, find_mro, lookup, obj_lookup, setup_func_call,
    update, update_class_env,
};

pub fn init_state(static_info: &Static) -> State {
    State {
        lineno: *static_info
            .statements
            .keys()
            .min()
            .expect("Atleast one statement should be present"),
        global_env: static_info.decvars[&0]
            .iter()
            .enumerate()
            .map(|(a, b)| (b.to_string(), a))
            .collect(),
        local_env: None,
        stack: vec![],
        store: vec![StorableValue::Bottom; static_info.decvars[&0].len()],
    }
}

pub fn tick(mut state: State, static_info: &Static) -> Option<State> {
    let lineno = state.lineno;
    let stmt = static_info.statements[&lineno];

    let mut next_state = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            if let Some(ast::ExprCall { func, args, .. }) = value.as_call_expr() {
                match *func.clone() {
                    ast::Expr::Name(ast::ExprName { id, .. }) => {
                        let mut lookup_env = state.local_env.clone().unwrap_or_default();
                        if let Some(Context::Class(_, class_env)) = state.stack.last() {
                            lookup_env.extend(class_env.clone());
                        }
                        let func_name = id.as_str();
                        let func_addr =
                            env_lookup(func_name, &Some(lookup_env), &state.global_env)?;
                        match state.store.get(func_addr).unwrap().clone() {
                            StorableValue::DefinitionClosure(func_lineno, func_env, formals) => {
                                if formals.len() != args.len() {
                                    panic!("Function call with wrong number of arguments");
                                }

                                let vals = args
                                    .iter()
                                    .map(|x| {
                                        eval(x, &state.local_env, &state.global_env, &state.store)
                                    })
                                    .collect::<Option<Vec<_>>>()?;

                                state.stack.push(Context::Lexical(lineno, state.local_env));

                                let (new_local_env, new_global_env, new_store) = setup_func_call(
                                    func_env,
                                    state.global_env,
                                    state.store,
                                    &static_info.decvars[&func_lineno],
                                    &static_info.globals[&func_lineno],
                                    formals,
                                    vals,
                                )?;

                                State {
                                    lineno: static_info.block[&func_lineno].0,
                                    local_env: Some(new_local_env),
                                    global_env: new_global_env,
                                    store: new_store,
                                    ..state
                                }
                            }
                            StorableValue::Object(Object {
                                metadata: ObjectMetadata { class: None, .. },
                                env_addr,
                                ..
                            }) => {
                                let class_env = state
                                    .store
                                    .get(env_addr)
                                    .and_then(|x| x.as_env().cloned())
                                    .expect("Object must have an initialized environment");

                                if let Some(StorableValue::DefinitionClosure(
                                    func_lineno,
                                    func_env,
                                    formals,
                                )) = lookup("__init__", &None, &class_env, &state.store).cloned()
                                {
                                    if formals.len() != args.len() + 1 {
                                        panic!("Function call with wrong number of arguments");
                                    }

                                    let mut vals = args
                                        .iter()
                                        .map(|x| {
                                            eval(
                                                x,
                                                &state.local_env,
                                                &state.global_env,
                                                &state.store,
                                            )
                                        })
                                        .collect::<Option<Vec<_>>>()?;

                                    let obj_env = BTreeMap::new();
                                    state.store.push(StorableValue::Env(obj_env));
                                    let obj = Object {
                                        metadata: ObjectMetadata {
                                            class: Some(func_addr),
                                            mro: None,
                                        },
                                        env_addr: state.store.len() - 1,
                                    };
                                    vals.insert(0, StorableValue::Object(obj));

                                    let return_closure = Context::Lexical(lineno, state.local_env);
                                    state.stack.push(return_closure);

                                    let (new_local_env, new_global_env, new_store) =
                                        setup_func_call(
                                            func_env,
                                            state.global_env,
                                            state.store,
                                            &static_info.decvars[&func_lineno],
                                            &static_info.globals[&func_lineno],
                                            formals,
                                            vals,
                                        )?;

                                    State {
                                        lineno: static_info.block[&func_lineno].0,
                                        local_env: Some(new_local_env),
                                        global_env: new_global_env,
                                        store: new_store,
                                        ..state
                                    }
                                } else {
                                    panic!("Class must have a __init__ function")
                                }
                            }
                            _ => panic!("Expected callable"),
                        }
                    }
                    ast::Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                        let obj_var = value
                            .as_name_expr()
                            .expect("Object fields must be accessed directly")
                            .id
                            .as_str();

                        let method = obj_lookup(
                            obj_var,
                            &attr,
                            &state.local_env,
                            &state.global_env,
                            &state.store,
                        );
                        if let Some(StorableValue::DefinitionClosure(
                            func_lineno,
                            func_env,
                            formals,
                        )) = method
                        {
                            if formals.len() != args.len() {
                                panic!("Function call with wrong number of arguments");
                            }

                            let vals = args
                                .iter()
                                .map(|x| eval(x, &state.local_env, &state.global_env, &state.store))
                                .collect::<Option<Vec<_>>>()?;

                            state.stack.push(Context::Lexical(lineno, state.local_env));

                            let (new_local_env, new_global_env, new_store) = setup_func_call(
                                func_env,
                                state.global_env,
                                state.store,
                                &static_info.decvars[&func_lineno],
                                &static_info.globals[&func_lineno],
                                formals,
                                vals,
                            )?;

                            State {
                                lineno: static_info.block[&func_lineno].0,
                                local_env: Some(new_local_env),
                                global_env: new_global_env,
                                store: new_store,
                                ..state
                            }
                        } else {
                            panic!("Method call but no associated method found")
                        }
                    }
                    _ => unimplemented!(),
                }
            } else {
                if let Some(Context::Class(_, class_env)) = state.stack.last_mut() {
                    state.store = assign_in_class_context(
                        &targets[0],
                        value,
                        &state.local_env,
                        &state.global_env,
                        class_env,
                        state.store,
                    )?;
                } else {
                    state.store = assign_in_lexical_context(
                        &targets[0],
                        value,
                        &state.local_env,
                        &state.global_env,
                        state.store,
                    )?;
                }

                State {
                    lineno: static_info.next_stmt[&lineno],
                    ..state
                }
            }
        }
        Stmt::While(ast::StmtWhile { test, .. }) | Stmt::If(ast::StmtIf { test, .. }) => {
            let res = eval(&test, &state.local_env, &state.global_env, &state.store)?;
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
        | Stmt::Pass(ast::StmtPass { .. })
        | Stmt::Global(ast::StmtGlobal { .. }) => State {
            lineno: static_info.next_stmt[&lineno],
            ..state
        },
        Stmt::FunctionDef(ast::StmtFunctionDef { name, args, .. }) => {
            let closure = StorableValue::DefinitionClosure(
                lineno,
                state.local_env.clone(),
                args.args.iter().map(|x| x.def.arg.to_string()).collect(),
            );

            if let Some(Context::Class(_, class_env)) = state.stack.last_mut() {
                state.store = update_class_env(name, closure, class_env, state.store);
            } else {
                state.store = update(
                    name,
                    closure,
                    &state.local_env,
                    &state.global_env,
                    state.store,
                )?;
            }

            State {
                lineno: static_info.next_stmt[&lineno],
                ..state
            }
        }
        Stmt::Return(ast::StmtReturn { value, .. }) => {
            let val = if let Some(expr) = value {
                eval(expr, &state.local_env, &state.global_env, &state.store)?
            } else {
                StorableValue::None
            };

            if let Some(Context::Lexical(ret_lineno, ret_env)) = state.stack.pop() {
                let targets = static_info.statements[&ret_lineno]
                    .as_assign_stmt()
                    .expect("Functions must be called in assignment statements")
                    .targets
                    .clone();

                if let Some(Context::Class(_, class_env)) = state.stack.last_mut() {
                    let mut lookup_env = ret_env.clone().unwrap_or_default();
                    lookup_env.extend(class_env.clone());
                    state.store = assign_val_in_class_context(
                        &targets[0],
                        val,
                        &Some(lookup_env),
                        &state.global_env,
                        class_env,
                        state.store,
                    )?;
                } else {
                    state.store = assign_val_in_lexical_context(
                        &targets[0],
                        val,
                        &ret_env,
                        &state.global_env,
                        state.store,
                    )?;
                }

                State {
                    lineno: static_info.next_stmt[&ret_lineno],
                    local_env: ret_env,
                    ..state
                }
            } else {
                panic!("Lexical context should be present at the top of the stack for a return")
            }
        }
        Stmt::ClassDef(ast::StmtClassDef { .. }) => {
            state.stack.push(Context::Class(lineno, BTreeMap::new()));
            State {
                lineno: static_info.block[&lineno].0,
                ..state
            }
        }
        Stmt::Expr(_) => todo!(),
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

    while let Some(Context::Class(class_lineno, class_env)) = next_state.stack.last().cloned() {
        if next_state.lineno == static_info.next_stmt[&static_info.block[&class_lineno].1] {
            next_state.stack.pop().unwrap();
            next_state.store.push(StorableValue::Env(class_env));
            let env_addr = next_state.store.len() - 1;
            let ast::StmtClassDef {
                name: class_name,
                bases,
                ..
            } = static_info.statements[&class_lineno]
                .as_class_def_stmt()
                .unwrap();
            let base_class_addrs: Vec<usize> =
                if let Some(Context::Class(_, class_env)) = next_state.stack.last() {
                    let mut lookup_env = next_state.local_env.clone().unwrap_or_default();
                    lookup_env.extend(class_env.clone());
                    bases
                        .iter()
                        .map(|base| {
                            env_lookup(
                                &base
                                    .as_name_expr()
                                    .expect("Base classes must be identifiers")
                                    .id,
                                &Some(lookup_env.clone()),
                                &next_state.global_env,
                            )
                            .expect("Base classes must be initialized")
                        })
                        .collect()
                } else {
                    bases
                        .iter()
                        .map(|base| {
                            env_lookup(
                                &base
                                    .as_name_expr()
                                    .expect("Base classes must be identifiers")
                                    .id,
                                &next_state.local_env,
                                &next_state.global_env,
                            )
                            .expect("Base classes must be initialized")
                        })
                        .collect()
                };
            let mut class_object = Object {
                metadata: ObjectMetadata {
                    class: None,
                    mro: None,
                },
                env_addr,
            };

            if let Some(Context::Class(_, class_env)) = next_state.stack.last_mut() {
                class_env
                    .entry(class_name.to_string())
                    .and_modify(|&mut class_idx| {
                        class_object.metadata.mro = Some(
                            find_mro(class_idx, base_class_addrs.clone(), &next_state.store)
                                .expect("MRO exists for class"),
                        );
                        next_state.store[class_idx] = StorableValue::Object(class_object.clone());
                    })
                    .or_insert_with(|| {
                        let class_idx = next_state.store.len();
                        class_object.metadata.mro = Some(
                            find_mro(class_idx, base_class_addrs.clone(), &next_state.store)
                                .expect("MRO exists for class"),
                        );
                        next_state.store.push(StorableValue::Object(class_object));
                        class_idx
                    });
            } else {
                let class_idx =
                    env_lookup(&class_name, &next_state.local_env, &next_state.global_env)?;
                class_object.metadata.mro = Some(
                    find_mro(class_idx, base_class_addrs.clone(), &next_state.store)
                        .expect("MRO exists for class"),
                );
                next_state.store = update(
                    class_name.as_str(),
                    StorableValue::Object(class_object),
                    &next_state.local_env,
                    &next_state.global_env,
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

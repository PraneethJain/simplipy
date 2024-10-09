use std::collections::BTreeMap;

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{Context, Object, State, StorableValue};
use crate::preprocess::Static;
use crate::utils::{
    assign_in_class_context, assign_in_lexical_context, assign_val_in_class_context,
    assign_val_in_lexical_context, eval, lookup, setup_func_call, update, update_class_env,
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
        local_env: BTreeMap::new(),
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
                let func_name = func.as_name_expr()?.id.as_str();
                match lookup(func_name, &state.local_env, &state.global_env, &state.store)?.clone()
                {
                    StorableValue::DefinitionClosure(func_lineno, func_env) => {
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
                            .map(|x| eval(x, &state.local_env, &state.global_env, &state.store))
                            .collect::<Option<Vec<_>>>()?;

                        state.stack.push(Context::Lexical(lineno, state.local_env));

                        let (new_env, new_store) = setup_func_call(
                            func_env,
                            &state.global_env,
                            state.store,
                            &static_info.decvars[&func_lineno],
                            formals,
                            vals,
                        )?;

                        State {
                            lineno: static_info.block[&func_lineno].0,
                            local_env: new_env,
                            store: new_store,
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

                        if let Some(StorableValue::DefinitionClosure(func_lineno, func_env)) =
                            lookup("__init__", &class_env, &BTreeMap::new(), &state.store).cloned()
                        {
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
                                .map(|x| eval(x, &state.local_env, &state.global_env, &state.store))
                                .collect::<Option<Vec<_>>>()?;

                            let obj_env = BTreeMap::new();
                            state.store.push(StorableValue::FlatEnv(obj_env));
                            let obj = Object {
                                class: Some(flat_env_addr),
                                flat_env_addr: state.store.len() - 1,
                            };
                            vals.insert(0, StorableValue::Object(obj));

                            let return_closure = Context::Lexical(lineno, state.local_env);
                            state.stack.push(return_closure);

                            let (new_env, new_store) = setup_func_call(
                                func_env,
                                &state.global_env,
                                state.store,
                                &static_info.decvars[&func_lineno],
                                formals,
                                vals,
                            )?;

                            State {
                                lineno: static_info.block[&func_lineno].0,
                                local_env: new_env,
                                store: new_store,
                                ..state
                            }
                        } else {
                            panic!("Class must have a __init__ function")
                        }
                    }
                    _ => panic!("Expected callable"),
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
        | Stmt::Pass(ast::StmtPass { .. }) => State {
            lineno: static_info.next_stmt[&lineno],
            ..state
        },
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => {
            let closure = StorableValue::DefinitionClosure(lineno, state.local_env.clone());

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
                    let mut lookup_env = ret_env.clone();
                    lookup_env.extend(class_env.clone());
                    state.store = assign_val_in_class_context(
                        &targets[0],
                        val,
                        &lookup_env,
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
        Stmt::ClassDef(ast::StmtClassDef { name, bases, .. }) => {
            state.stack.push(Context::Class(lineno, BTreeMap::new()));
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

    while let Some(Context::Class(class_lineno, class_env)) = next_state.stack.last().cloned() {
        if next_state.lineno == static_info.next_stmt[&static_info.block[&class_lineno].1] {
            next_state.stack.pop().unwrap();
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

            if let Some(Context::Class(_, class_env)) = next_state.stack.last_mut() {
                class_env
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

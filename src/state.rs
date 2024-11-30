use std::collections::BTreeMap;

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{State, StorableValue};
use crate::preprocess::Static;
use crate::utils::{eval, find_lexical_block, lookup, setup_func_call, update};

pub fn init_state(static_info: &Static) -> State {
    State {
        envs: BTreeMap::from([(0, BTreeMap::new())]),
        parent: BTreeMap::new(),
        stack: vec![(
            *static_info
                .statements
                .keys()
                .min()
                .expect("Atleast one statement should be present"),
            0,
        )],
    }
}

pub fn tick(mut state: State, static_info: &Static) -> Option<State> {
    let (lineno, env_id) = state.stack.last().unwrap().clone();
    let stmt = static_info.statements[&lineno];

    let lexical_block_lineno = find_lexical_block(lineno, &static_info.block).unwrap();
    let globals = &static_info.globals.get(&lexical_block_lineno).unwrap();

    let next_state = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            if let Some(ast::ExprCall { func, args, .. }) = value.as_call_expr() {
                match *func.clone() {
                    ast::Expr::Name(ast::ExprName { id, .. }) => {
                        let func =
                            lookup(id.as_str(), env_id, &state.envs, &state.parent, globals)?;
                        match func {
                            StorableValue::DefinitionClosure(
                                func_lineno,
                                parent_env_id,
                                formals,
                            ) => {
                                if formals.len() != args.len() {
                                    panic!("Function call with wrong number of arguments");
                                }

                                let vals = args
                                    .iter()
                                    .map(|x| eval(x, env_id, &state.envs, &state.parent, globals))
                                    .collect::<Option<Vec<_>>>()?;

                                let new_envs = setup_func_call(
                                    state.envs,
                                    &static_info.decvars[&func_lineno],
                                    formals,
                                    vals,
                                );

                                state
                                    .stack
                                    .push((static_info.block[&func_lineno].0, new_envs.len() - 1));
                                let mut new_parent = state.parent;
                                new_parent.insert(new_envs.len() - 1, parent_env_id);

                                State {
                                    envs: new_envs,
                                    parent: new_parent,
                                    ..state
                                }
                            }
                            _ => panic!("Expected callable"),
                        }
                    }
                    _ => unimplemented!(),
                }
            } else {
                let val = eval(value, env_id, &state.envs, &state.parent, globals)?;
                let var = &targets[0].as_name_expr().unwrap().id;

                let new_envs = if globals.contains(var.as_str()) {
                    state.envs.get_mut(&0).unwrap().insert(var.to_string(), val);
                    state.envs
                } else {
                    update(var, val, env_id, state.envs, &state.parent)
                };

                let (lineno, env_id) = state.stack.pop().unwrap();
                state.stack.push((static_info.next_stmt[&lineno], env_id));

                State {
                    envs: new_envs,
                    ..state
                }
            }
        }
        Stmt::While(ast::StmtWhile { test, .. }) | Stmt::If(ast::StmtIf { test, .. }) => {
            let res = eval(&test, env_id, &state.envs, &state.parent, globals)?;
            let bool_res = res.bool()?;

            let (lineno, env_id) = state.stack.pop().unwrap();
            state.stack.push((
                if bool_res {
                    static_info.true_stmt[&lineno]
                } else {
                    static_info.false_stmt[&lineno]
                },
                env_id,
            ));

            State { ..state }
        }
        Stmt::Continue(ast::StmtContinue { .. })
        | Stmt::Break(ast::StmtBreak { .. })
        | Stmt::Pass(ast::StmtPass { .. })
        | Stmt::Global(ast::StmtGlobal { .. })
        | Stmt::Nonlocal(ast::StmtNonlocal { .. }) => {
            let (lineno, env_id) = state.stack.pop().unwrap();
            state.stack.push((static_info.next_stmt[&lineno], env_id));
            state
        }
        Stmt::FunctionDef(ast::StmtFunctionDef { name, args, .. }) => {
            let closure = StorableValue::DefinitionClosure(
                lineno,
                env_id,
                args.args.iter().map(|x| x.def.arg.to_string()).collect(),
            );

            let new_envs = if globals.contains(name.as_str()) {
                state
                    .envs
                    .get_mut(&0)
                    .unwrap()
                    .insert(name.to_string(), closure);
                state.envs
            } else {
                update(name, closure, env_id, state.envs, &state.parent)
            };

            let (lineno, env_id) = state.stack.pop().unwrap();
            state.stack.push((static_info.next_stmt[&lineno], env_id));

            State {
                envs: new_envs,
                ..state
            }
        }
        Stmt::Return(ast::StmtReturn { value, .. }) => {
            let val = if let Some(expr) = value {
                eval(expr, env_id, &state.envs, &state.parent, globals)?
            } else {
                StorableValue::None
            };

            if let Some((_, _)) = state.stack.pop() {
                let (ret_lineno, ret_env_id) = state.stack.last().unwrap().clone();
                let targets = static_info.statements[&ret_lineno]
                    .as_assign_stmt()
                    .expect("Functions must be called in assignment statements")
                    .targets
                    .clone();

                let var = &targets[0].as_name_expr().unwrap().id;
                let ret_lexical_block = find_lexical_block(ret_lineno, &static_info.block).unwrap();
                let ret_globals = &static_info.globals.get(&ret_lexical_block).unwrap();
                let new_envs = if ret_globals.contains(var.as_str()) {
                    state.envs.get_mut(&0).unwrap().insert(var.to_string(), val);
                    state.envs
                } else {
                    update(var, val, ret_env_id, state.envs, &state.parent)
                };

                let (lineno, env_id) = state.stack.pop().unwrap();
                state.stack.push((static_info.next_stmt[&lineno], env_id));

                State {
                    envs: new_envs,
                    ..state
                }
            } else {
                panic!("Lexical context should be present at the top of the stack for a return")
            }
        }
        Stmt::ClassDef(_) => todo!(),
        Stmt::Expr(_) => todo!(),
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
    Some(State { ..next_state })
}

pub fn is_fixed_point(state: &State, static_info: &Static) -> bool {
    let lineno = state.stack.last().unwrap().0;
    static_info.next_stmt[&lineno] == lineno
}

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{Closure, Env, Stack, StorableValue, Store};
use crate::preprocess::Static;
use crate::utils::{eval, lookup, update};

#[derive(Debug)]
pub struct State {
    pub lineno: u64,
    pub env: Env,
    pub stack: Stack,
    pub store: Store,
}

pub fn init_state(static_info: &Static) -> State {
    State {
        lineno: *static_info
            .statements
            .keys()
            .min()
            .expect("Atleast one statement should be present"),
        env: vec![static_info.decvars[&0]
            .iter()
            .enumerate()
            .map(|(a, b)| (b.to_string(), a))
            .collect()],
        stack: vec![],
        store: vec![StorableValue::Bottom; static_info.decvars[&0].len()],
    }
}

pub fn tick(mut state: State, static_info: &Static) -> Option<State> {
    let lineno = state.lineno;
    let stmt = static_info.statements[&lineno];

    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            let var = targets[0]
                .as_name_expr()
                .expect("Expected simple assignment")
                .id
                .as_str();

            if let Some(ast::ExprCall { func, args, .. }) = value.as_call_expr() {
                let func_name = func.as_name_expr()?.id.as_str();
                if let Closure::Function(func_lineno, func_env) =
                    lookup(func_name, &state.env, &state.store)?
                        .clone()
                        .as_closure()?
                {
                    let func_stmt = static_info.statements[&func_lineno].as_function_def_stmt()?;
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

                    let return_closure = Closure::Return(lineno, state.env);
                    state.stack.push(return_closure);

                    let n = state.store.len();
                    state.env = func_env;
                    state.env.push(
                        static_info.decvars[&func_lineno]
                            .iter()
                            .enumerate()
                            .map(|(i, x)| (x.to_string(), n + i))
                            .collect(),
                    );
                    state.store.extend(vec![
                        StorableValue::Bottom;
                        static_info.decvars[&func_lineno].len()
                    ]);

                    for (formal, val) in formals.into_iter().zip(vals.into_iter()) {
                        state.store = update(formal, val, &state.env, state.store)?;
                    }

                    let func_body_lineno = *if let Some((func_body_lineno, _)) = static_info
                        .statements
                        .range((
                            std::ops::Bound::Excluded(func_lineno),
                            std::ops::Bound::Unbounded,
                        ))
                        .next()
                    {
                        func_body_lineno
                    } else {
                        panic!("Function body cannot be empty");
                    };

                    Some(State {
                        lineno: func_body_lineno,
                        ..state
                    })
                } else {
                    panic!("Function called but closure is not a function closure");
                }
            } else {
                let val = eval(&value, &state.env, &state.store)?;
                let new_store = update(var, val, &state.env, state.store)?;
                Some(State {
                    lineno: static_info.next_stmt[&lineno],
                    store: new_store,
                    ..state
                })
            }
        }
        Stmt::While(ast::StmtWhile { test, .. }) | Stmt::If(ast::StmtIf { test, .. }) => {
            let res = eval(&test, &state.env, &state.store)?;
            let bool_res = res.as_bool()?;
            Some(State {
                lineno: if bool_res {
                    static_info.true_stmt[&lineno]
                } else {
                    static_info.false_stmt[&lineno]
                },
                ..state
            })
        }
        Stmt::Continue(ast::StmtContinue { .. }) | Stmt::Break(ast::StmtBreak { .. }) => {
            Some(State {
                lineno: static_info.next_stmt[&lineno],
                ..state
            })
        }
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => {
            let closure = StorableValue::Closure(Closure::Function(lineno, state.env.clone()));
            let new_store = update(name.as_str(), closure, &state.env, state.store)?;
            Some(State {
                lineno: static_info.next_stmt[&lineno],
                store: new_store,
                ..state
            })
        }
        Stmt::Return(ast::StmtReturn { value, .. }) => {
            let val = if let Some(expr) = value {
                eval(expr, &state.env, &state.store)?
            } else {
                StorableValue::None
            };

            if let Closure::Return(ret_lineno, ret_env) = state
                .stack
                .pop()
                .expect("Non empty stack during function return")
            {
                let var = static_info.statements[&ret_lineno]
                    .as_assign_stmt()
                    .expect("Functions must be called in assignment statements")
                    .targets[0]
                    .as_name_expr()
                    .expect("Assignments must be simple")
                    .id
                    .as_str();

                state.store = update(var, val, &state.env, state.store)?;

                Some(State {
                    lineno: static_info.next_stmt[&ret_lineno],
                    env: ret_env,
                    ..state
                })
            } else {
                panic!("Return but closure is not a return closure");
            }
        }
        Stmt::ClassDef(_) => todo!(),
        Stmt::Expr(_) => todo!(),
        Stmt::Pass(_) => todo!(),
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
    }
}

pub fn is_fixed_point(state: &State, static_info: &Static) -> bool {
    let lineno = state.lineno;

    !(static_info.true_stmt.contains_key(&lineno)
        || static_info.false_stmt.contains_key(&lineno)
        || static_info.next_stmt.contains_key(&lineno))
}

use rustpython_parser::ast::{self, Stmt};

use crate::datatypes::{Env, Stack, StorableValue, Store};
use crate::preprocess::Static;
use crate::utils::{eval, update};

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

pub fn tick(state: State, static_info: &Static) -> Option<State> {
    let lineno = state.lineno;
    let stmt = static_info.statements[&lineno];

    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            let var = targets[0]
                .as_name_expr()
                .expect("Expected simple assignment")
                .id
                .as_str();

            let val = eval(&value, &state.env, &state.store)?;
            let new_store = update(var, val, &state.env, state.store)?;
            Some(State {
                lineno: static_info.next_stmt[&lineno],
                env: state.env,
                stack: state.stack,
                store: new_store,
            })
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
        Stmt::FunctionDef(ast::StmtFunctionDef { .. }) => todo!(),
        Stmt::Return(_) => todo!(),
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

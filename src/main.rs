mod preprocess;
use preprocess::preprocess_module;
use preprocess::Static;

mod datatypes;
use datatypes::{Env, Stack, StorableValue, Store};

mod utils;
use utils::{eval, update};

use rustpython_parser::{
    ast::{self, source_code::LineIndex, Stmt},
    parse, Mode,
};
use std::{env, fs};

#[derive(Debug)]
struct State {
    lineno: u64,
    env: Env,
    stack: Stack,
    store: Store,
}
fn tick(state: State, static_info: &Static) -> State {
    let lineno = state.lineno;
    let stmt = static_info.statements[&lineno];

    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            let var = targets[0]
                .as_name_expr()
                .expect("Expected simple assignment")
                .id
                .as_str();

            if let Some(val) = eval(value, &state.env, &state.store) {
                if let Some(new_store) = update(var, val, &state.env, state.store) {
                    State {
                        lineno: static_info.next_stmt[&lineno],
                        env: state.env,
                        stack: state.stack,
                        store: new_store,
                    }
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        }
        Stmt::While(ast::StmtWhile { .. }) => todo!(),
        Stmt::If(ast::StmtIf { .. }) => todo!(),
        Stmt::Continue(ast::StmtContinue { .. }) => todo!(),
        Stmt::Break(ast::StmtBreak { .. }) => todo!(),
        Stmt::FunctionDef(ast::StmtFunctionDef { .. }) => todo!(),
        Stmt::ClassDef(_) => todo!(),
        Stmt::Return(_) => todo!(),
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

fn is_fixed_point(state: &State, static_info: &Static) -> bool {
    let lineno = state.lineno;

    !(static_info.true_stmt.contains_key(&lineno)
        || static_info.false_stmt.contains_key(&lineno)
        || static_info.next_stmt.contains_key(&lineno))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <python_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let source = fs::read_to_string(filename)?;

    let ast = parse(&source, Mode::Module, "<embedded>")?;
    let line_index = LineIndex::from_source_text(&source);

    let module = ast.as_module().expect("Must be a python module");
    let static_info = preprocess_module(module, &line_index, &source);

    let init_state = State {
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
    };

    println!("{:?}", ast);

    let mut cur_state = init_state;
    while !is_fixed_point(&cur_state, &static_info) {
        cur_state = tick(cur_state, &static_info);
        println!("{}: {:?}", cur_state.lineno, cur_state);
    }

    Ok(())
}

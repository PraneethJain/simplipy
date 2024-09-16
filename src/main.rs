use rustpython_parser::{ast::source_code::LineIndex, parse, Mode};
use std::{env, fs};

mod datatypes;
mod preprocess;
mod state;
mod utils;

use preprocess::preprocess_module;
use state::{init_state, is_fixed_point, tick};

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

    println!("{:?}", ast);

    let mut cur_state = init_state(&static_info);
    while !is_fixed_point(&cur_state, &static_info) {
        cur_state = tick(cur_state, &static_info).expect("Valid transition");
        println!("{}: {:?}", cur_state.lineno, cur_state);
    }

    Ok(())
}

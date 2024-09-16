mod preprocess;
use preprocess::preprocess_module;
use preprocess::Static;

use rustpython_parser::{ast::source_code::LineIndex, parse, Mode};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <python_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let source = fs::read_to_string(filename)?;

    let ast = parse(&source, Mode::Module, "")?;
    let line_index = LineIndex::from_source_text(&source);

    let module = ast.as_module().expect("Must be a python module");
    let Static {
        statements,
        next_stmt,
        true_stmt,
        false_stmt,
        decvars,
        ..
    } = preprocess_module(module, &line_index, &source);

    println!("statements: {:?}", statements.keys());
    println!("--------------------------------------------------");
    println!("next: {:?}", next_stmt);
    println!("--------------------------------------------------");
    println!("true: {:?}", true_stmt);
    println!("--------------------------------------------------");
    println!("false: {:?}", false_stmt);
    println!("--------------------------------------------------");
    println!("decvars: {:?}", decvars);

    Ok(())
}


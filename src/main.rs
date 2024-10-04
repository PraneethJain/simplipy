use rustpython_parser::{ast::source_code::LineIndex, parse, Mode};
use std::{env, fs};



use simplipy::app::App;
use simplipy::preprocess::preprocess_module;

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

    let mut terminal = ratatui::init();
    let _ = App::new(&source, &static_info).run(&mut terminal);
    ratatui::restore();

    Ok(())
}

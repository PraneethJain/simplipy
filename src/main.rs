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
        ..
    } = preprocess_module(module, &line_index, &source);

    println!("statements: {:?}", statements.keys());
    println!("--------------------------------------------------");
    println!("next: {:?}", next_stmt);
    println!("--------------------------------------------------");
    println!("true: {:?}", true_stmt);
    println!("--------------------------------------------------");
    println!("false: {:?}", false_stmt);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_sequential() {
        let source = r#"
x = 3
y = 6
z = x + y
"#;
        let ast = parse(source, Mode::Module, "<embedded>").unwrap();
        let line_index = LineIndex::from_source_text(source);
        let module = ast.as_module().unwrap();
        let Static { next_stmt, .. } = preprocess_module(module, &line_index, &source);
        next_stmt.iter().for_each(|(&a, &b)| assert_eq!(a + 1, b));
    }

    #[test]
    fn if_statement() {
        let source = r#"            # 1
x = 3                               # 2
y = 6                               # 3
if True:                            # 4
    if True:                        # 5
        if True:                    # 6
            z = x + y               # 7
        else:                       # 8
            y = x                   # 9
            if False:               # 10
                if True:            # 11
                    z = 2           # 12
                else:               # 13
                    z = 3           # 14
                                    # 15
if False:                           # 16
    if True:                        # 17
        z = 10                      # 18
    else:                           # 19
        z = 20                      # 20
                                    # 21
z = x + y                           # 22
"#;

        let ast = parse(source, Mode::Module, "<embedded>").unwrap();
        let line_index = LineIndex::from_source_text(source);
        let module = ast.as_module().unwrap();
        let Static {
            next_stmt,
            true_stmt,
            false_stmt,
            ..
        } = preprocess_module(module, &line_index, &source);

        for (cur, next) in [
            (2, 3),
            (3, 4),
            (7, 16),
            (9, 10),
            (12, 16),
            (14, 16),
            (18, 22),
            (20, 22),
        ] {
            assert_eq!(next_stmt[&cur], next);
        }

        for (cur, next) in [
            (4, 5),
            (5, 6),
            (6, 7),
            (10, 11),
            (11, 12),
            (16, 17),
            (17, 18),
        ] {
            assert_eq!(true_stmt[&cur], next);
        }

        for (cur, next) in [
            (4, 16),
            (5, 16),
            (6, 9),
            (10, 16),
            (11, 14),
            (16, 22),
            (17, 20),
        ] {
            assert_eq!(false_stmt[&cur], next);
        }
    }

    #[test]
    fn while_with_if_statement() {
        let source = r#"
x = 3
y = 6
while True:
    while True:
        while True:
            z = x + y
            y = x
            if False:
                if True:
                    z = 2
                else:
                    z = 3
            continue
        continue
    continue

while False:              
    if True:
        z = 10
    else:
        z = 20
    continue

z = x + y           
"#;

        let ast = parse(source, Mode::Module, "<embedded>").unwrap();
        let line_index = LineIndex::from_source_text(source);
        let module = ast.as_module().unwrap();
        let Static {
            next_stmt,
            true_stmt,
            false_stmt,
            ..
        } = preprocess_module(module, &line_index, &source);

        println!("{:?}", next_stmt);

        for (cur, next) in [
            (2, 3),
            (3, 4),
            (7, 8),
            (8, 9),
            (11, 14),
            (13, 14),
            (14, 6),
            (15, 5),
            (16, 4),
            (20, 23),
            (22, 23),
            (23, 18),
        ] {
            println!("{}", cur);
            assert_eq!(next_stmt[&cur], next);
        }

        for (cur, next) in [
            (4, 5),
            (5, 6),
            (6, 7),
            (9, 10),
            (10, 11),
            (18, 19),
            (19, 20),
        ] {
            assert_eq!(true_stmt[&cur], next);
        }

        for (cur, next) in [
            (4, 18),
            (5, 16),
            (6, 15),
            (9, 14),
            (10, 13),
            (18, 25),
            (19, 22),
        ] {
            assert_eq!(false_stmt[&cur], next);
        }
    }

    #[test]
    fn while_with_break() {
        let source = r#"
while True:
    break
    continue

while True:
    while True:
        break
        continue
    break
    continue

z = 4
"#;

        let ast = parse(source, Mode::Module, "<embedded>").unwrap();
        let line_index = LineIndex::from_source_text(source);
        let module = ast.as_module().unwrap();
        let Static {
            next_stmt,
            true_stmt,
            false_stmt,
            ..
        } = preprocess_module(module, &line_index, &source);

        println!("{:?}", next_stmt);

        for (cur, next) in [(3, 6), (4, 2), (8, 10), (9, 7), (10, 13), (11, 6)] {
            println!("{}", cur);
            assert_eq!(next_stmt[&cur], next);
        }

        for (cur, next) in [(2, 3), (6, 7), (7, 8)] {
            assert_eq!(true_stmt[&cur], next);
        }

        for (cur, next) in [(2, 6), (6, 13), (7, 10)] {
            assert_eq!(false_stmt[&cur], next);
        }
    }
}

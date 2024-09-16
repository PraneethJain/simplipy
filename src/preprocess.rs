use rustpython_parser::ast::{self, source_code::LineIndex, ModModule, Ranged, Stmt};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! get_current_line {
    ($line_index:expr, $stmt:expr, $source:expr) => {
        $line_index
            .source_location($stmt.start(), $source)
            .row
            .get() as u64
    };
}

#[derive(Debug, Default)]
pub struct Static<'a> {
    pub statements: BTreeMap<u64, &'a Stmt>,
    pub next_stmt: BTreeMap<u64, u64>,
    pub true_stmt: BTreeMap<u64, u64>,
    pub false_stmt: BTreeMap<u64, u64>,
    pub decvars: BTreeMap<u64, BTreeSet<&'a str>>,
    parent_map: BTreeMap<u64, u64>,
    cur_scope_lineno: u64,
}

pub fn preprocess_module<'a>(
    module: &'a ModModule,
    line_index: &LineIndex,
    source: &str,
) -> Static<'a> {
    let mut static_info = Static::default();
    static_info.decvars.insert(0, BTreeSet::new());

    traverse_module(module, line_index, source, &mut static_info);

    static_info
}

fn new_block<'a>(
    body: &'a [Stmt],
    line_index: &LineIndex,
    source: &str,
) -> (Vec<(u64, &'a Stmt)>, Vec<(u64, u64)>) {
    let new_statements: Vec<_> = body
        .iter()
        .map(|stmt| (get_current_line!(line_index, stmt, source), stmt))
        .collect();

    let new_next_stmts: Vec<_> = new_statements
        .iter()
        .map(|(a, _)| a)
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (*w[0], *w[1]))
        .collect();

    (new_statements, new_next_stmts)
}

fn traverse_module<'a, 'b>(
    module: &'a ModModule,
    line_index: &LineIndex,
    source: &str,
    static_info: &'b mut Static<'a>,
) {
    let (new_statements, new_next_stmts) = new_block(&module.body, line_index, source);
    static_info.statements.extend(new_statements);
    static_info.next_stmt.extend(new_next_stmts);
    for inner_stmt in &module.body {
        traverse_stmt(inner_stmt, line_index, source, static_info);
    }
}

fn traverse_body<'a, 'b>(
    parent_lineno: u64,
    body: &'a [Stmt],
    line_index: &LineIndex,
    source: &str,
    static_info: &'b mut Static<'a>,
) {
    let (new_statements, new_next_stmts) = new_block(body, line_index, source);
    static_info.statements.extend(new_statements);
    static_info.next_stmt.extend(new_next_stmts);

    if let Some(&parent_next_lineno) = static_info.next_stmt.get(&parent_lineno) {
        if let Some(last_stmt) = body.last() {
            static_info.next_stmt.insert(
                get_current_line!(line_index, last_stmt, source),
                parent_next_lineno,
            );
        }
    }

    for inner_stmt in body {
        let inner_lineno = get_current_line!(line_index, inner_stmt, source);
        static_info.parent_map.insert(inner_lineno, parent_lineno);
        traverse_stmt(inner_stmt, line_index, source, static_info);
    }
}

fn traverse_stmt<'a, 'b>(
    stmt: &'a Stmt,
    line_index: &LineIndex,
    source: &str,
    static_info: &'b mut Static<'a>,
) {
    let cur_lineno = get_current_line!(line_index, stmt, source);
    match stmt {
        Stmt::While(ast::StmtWhile { body, .. }) => {
            let true_lineno = get_current_line!(
                line_index,
                body.first().expect("While body should be non-empty"),
                source
            );
            static_info.true_stmt.insert(cur_lineno, true_lineno);
            if let Some(&false_lineno) = static_info.next_stmt.get(&cur_lineno).or_else(|| {
                static_info
                    .parent_map
                    .get(&cur_lineno)
                    .and_then(|&parent_lineno| static_info.next_stmt.get(&parent_lineno))
            }) {
                static_info.false_stmt.insert(cur_lineno, false_lineno);
            }

            traverse_body(cur_lineno, body, line_index, source, static_info);
        }
        Stmt::If(ast::StmtIf { body, orelse, .. }) => {
            let true_lineno = get_current_line!(
                line_index,
                body.first().expect("If body should be non-empty"),
                source
            );
            static_info.true_stmt.insert(cur_lineno, true_lineno);

            let false_lineno = if let Some(orelse_stmt) = orelse.first() {
                Some(get_current_line!(line_index, orelse_stmt, source))
            } else {
                static_info.next_stmt.get(&cur_lineno).copied().or_else(|| {
                    static_info
                        .parent_map
                        .get(&cur_lineno)
                        .and_then(|&parent_lineno| static_info.next_stmt.get(&parent_lineno))
                        .copied()
                })
            };

            if let Some(false_lineno) = false_lineno {
                static_info.false_stmt.insert(cur_lineno, false_lineno);
            }

            traverse_body(cur_lineno, body, line_index, source, static_info);
            traverse_body(cur_lineno, orelse, line_index, source, static_info);
        }
        Stmt::Continue(ast::StmtContinue { .. }) => {
            let mut lineno = cur_lineno;
            loop {
                if let Some(&parent_lineno) = static_info.parent_map.get(&lineno) {
                    if let Stmt::While(_) = static_info.statements[&parent_lineno] {
                        static_info.next_stmt.insert(cur_lineno, parent_lineno);
                        break;
                    } else {
                        lineno = parent_lineno;
                    }
                } else {
                    panic!("continue found outside loop at lineno: {}", cur_lineno);
                }
            }
        }
        Stmt::Break(ast::StmtBreak { .. }) => {
            let mut lineno = cur_lineno;
            loop {
                if let Some(&parent_lineno) = static_info.parent_map.get(&lineno) {
                    if let Stmt::While(_) = static_info.statements[&parent_lineno] {
                        if let Some(&parent_false_lineno) =
                            static_info.false_stmt.get(&parent_lineno)
                        {
                            static_info
                                .next_stmt
                                .insert(cur_lineno, parent_false_lineno);
                        } else {
                            // program has terminated
                        }
                        break;
                    } else {
                        lineno = parent_lineno;
                    }
                } else {
                    panic!("break found outside loop at lineno: {}", cur_lineno);
                }
            }
        }

        Stmt::FunctionDef(ast::StmtFunctionDef {
            name, args, body, ..
        }) => {
            static_info
                .decvars
                .get_mut(&static_info.cur_scope_lineno)
                .expect("decvars must be created for the scope before assignment")
                .insert(name.as_str());

            let old_scope_lineno = static_info.cur_scope_lineno;
            static_info.cur_scope_lineno = cur_lineno;

            static_info.decvars.insert(
                cur_lineno,
                BTreeSet::from_iter(args.args.iter().map(|x| x.def.arg.as_str())),
            );
            traverse_body(cur_lineno, body, line_index, source, static_info);
            static_info.cur_scope_lineno = old_scope_lineno;
        }
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            if targets.len() != 1 {
                panic!("Expected simple assignment");
            }
            let var = targets[0]
                .as_name_expr()
                .expect("Expected simple assignment")
                .id
                .as_str();
            static_info
                .decvars
                .get_mut(&static_info.cur_scope_lineno)
                .expect("decvars must be created for the scope before assignment")
                .insert(var);
        }
        _ => {} // Stmt::ClassDef(_) => todo!(),
                // Stmt::Return(_) => todo!(),
                // Stmt::Expr(_) => todo!(),
                // Stmt::Pass(_) => todo!(),
                // Stmt::Global(_) => todo!(),
                // Stmt::Nonlocal(_) => todo!(),
                // Stmt::Import(_) => todo!(),
                // Stmt::ImportFrom(_) => todo!(),
                // Stmt::Try(_) => todo!(),
                // Stmt::TryStar(_) => todo!(),
                // Stmt::Raise(_) => todo!(),
                // Stmt::AugAssign(_) => unimplemented!(),
                // Stmt::For(_) => unimplemented!(),
                // Stmt::AsyncFunctionDef(_) => unimplemented!(),
                // Stmt::AnnAssign(_) => unimplemented!(),
                // Stmt::Assert(_) => unimplemented!(),
                // Stmt::With(_) => unimplemented!(),
                // Stmt::AsyncWith(_) => unimplemented!(),
                // Stmt::AsyncFor(_) => unimplemented!(),
                // Stmt::Match(_) => unimplemented!(),
                // Stmt::TypeAlias(_) => unimplemented!(),
                // Stmt::Delete(_) => unimplemented!(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rustpython_parser::{parse, Mode};

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
        let Static {
            next_stmt, decvars, ..
        } = preprocess_module(module, &line_index, &source);
        next_stmt.iter().for_each(|(&a, &b)| assert_eq!(a + 1, b));
        assert_eq!(decvars[&0], BTreeSet::from(["x", "y", "z"]));
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
            decvars,
            ..
        } = preprocess_module(module, &line_index, &source);

        assert_eq!(decvars[&0], BTreeSet::from(["x", "y", "z"]));

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
            decvars,
            ..
        } = preprocess_module(module, &line_index, &source);

        assert_eq!(decvars[&0], BTreeSet::from(["x", "y", "z"]));

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
            decvars,
            ..
        } = preprocess_module(module, &line_index, &source);

        assert_eq!(decvars[&0], BTreeSet::from(["z"]));

        for (cur, next) in [(3, 6), (4, 2), (8, 10), (9, 7), (10, 13), (11, 6)] {
            assert_eq!(next_stmt[&cur], next);
        }

        for (cur, next) in [(2, 3), (6, 7), (7, 8)] {
            assert_eq!(true_stmt[&cur], next);
        }

        for (cur, next) in [(2, 6), (6, 13), (7, 10)] {
            assert_eq!(false_stmt[&cur], next);
        }
    }

    #[test]
    fn function_with_while() {
        let source = r#"
def f(x ,y):
    a = 2
    while True:
        break
        continue
    def g(z):
        return x + y + z

    return g

x = f()
y = x()

print(y)
"#;
        let ast = parse(source, Mode::Module, "<embedded>").unwrap();
        let line_index = LineIndex::from_source_text(source);
        let module = ast.as_module().unwrap();
        let Static {
            next_stmt,
            true_stmt,
            false_stmt,
            decvars,
            ..
        } = preprocess_module(module, &line_index, &source);

        assert_eq!(decvars[&0], BTreeSet::from(["f", "x", "y"]));
        assert_eq!(decvars[&2], BTreeSet::from(["a", "x", "y", "g"]));
        assert_eq!(decvars[&7], BTreeSet::from(["z"]));

        for (cur, next) in [(2, 12), (3, 4), (5, 7), (6, 4), (7, 10), (12, 13), (13, 15)] {
            assert_eq!(next_stmt[&cur], next);
        }

        for (cur, next) in [(4, 5)] {
            assert_eq!(true_stmt[&cur], next);
        }

        for (cur, next) in [(4, 7)] {
            assert_eq!(false_stmt[&cur], next);
        }
    }
}
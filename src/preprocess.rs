use rustpython_parser::ast::{self, source_code::LineIndex, ModModule, Ranged, Stmt};
use std::collections::BTreeMap;

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
    parent_map: BTreeMap<u64, u64>,
}

pub fn preprocess_module<'a>(
    module: &'a ModModule,
    line_index: &LineIndex,
    source: &str,
) -> Static<'a> {
    let mut static_info = Static::default();

    traverse_module(module, line_index, source, &mut static_info);

    println!("Parent Map: {:?}", static_info.parent_map);

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

        _ => {} // Stmt::FunctionDef(..) => todo!(),
                // Stmt::ClassDef(_) => todo!(),
                // Stmt::Return(_) => todo!(),
                // Stmt::Assign(_) => todo!(),
                // Stmt::Expr(_) => todo!(),
                // Stmt::Pass(_) => todo!(),
                // Stmt::Break(_) => todo!(),
                // Stmt::Continue(_) => todo!(),
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

use sqlparser::ast::{ColumnDef, Statement};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::collections::HashSet;

/// Compares two SQL strings for semantic equivalence, even if fields
/// are ordered differently.
/// Arguments
/// * `lhs` - a sql string
/// * `rhs` - a different sql string.
pub fn is_sql_equivalent(lhs: &str, rhs: &str) -> bool {
    let dialect = PostgreSqlDialect {}; // or AnsiDialect

    let built_ast = &Parser::parse_sql(&dialect, lhs).unwrap()[0];
    let expected_ast = &Parser::parse_sql(&dialect, rhs).unwrap()[0];

    // Because of the stupid non-deterministic nature of how the sql generation works, we
    // have to compare the members of their parsed ASTs.
    if let Statement::CreateTable { columns, .. } = built_ast {
        let built_columns = HashSet::<ColumnDef>::from_iter(columns.iter().cloned());

        if let Statement::CreateTable { columns, .. } = expected_ast {
            let expected_columns = HashSet::<ColumnDef>::from_iter(columns.iter().cloned());
            return expected_columns == built_columns;
        }
    } else {
        eprintln!("Don't know how to check {:#?}", built_ast);
    }
    false
}

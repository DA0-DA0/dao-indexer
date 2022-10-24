use convert_case::{Case, Casing};

pub static DEFAULT_ID_COLUMN_NAME: &str = "id";
pub static DEFAULT_TABLE_NAME_COLUMN_NAME: &str = "target_table_name";
pub static TARGET_ID_COLUMN_NAME: &str = "target_id";

fn db_normalize(input: &str) -> String {
    input.to_case(Case::Snake)
}

pub fn db_table_name(input_name: &str) -> String {
    db_normalize(input_name)
}

pub fn db_column_name(input_name: &str) -> String {
    db_normalize(input_name)
}

pub fn foreign_key(column_name: &str) -> String {
    format!("{}_id", db_column_name(column_name))
}

pub fn constraint_name(table_name: &str, column_name: &str) -> String {
    format!(
        "{}_{}_fkey",
        db_table_name(table_name),
        db_column_name(column_name)
    )
}

use convert_case::{Case, Casing};

pub static DEFAULT_ID_COLUMN_NAME: &str = "id";
pub static DEFAULT_TABLE_NAME_COLUMN_NAME: &str = "table_name";

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

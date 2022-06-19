use sea_orm::sea_query::{
    Alias, ColumnDef, /* ForeignKey, ForeignKeyAction,*/ Table, TableCreateStatement,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DatabaseBuilder {
    tables: HashMap<String, TableCreateStatement>,
    columns: HashMap<String, HashMap<String, ColumnDef>>,
}

impl DatabaseBuilder {
    pub fn new() -> Self {
        DatabaseBuilder {
            tables: HashMap::new(),
            columns: HashMap::new(),
        }
    }
    pub fn table(&mut self, table_name: &str) -> &mut TableCreateStatement {
        self.tables
            .entry(table_name.to_string())
            .or_insert_with(|| {
                Table::create()
                    .table(Alias::new(table_name))
                    .if_not_exists()
                    .to_owned()
            })
    }

    pub fn column(&mut self, table_name: &str, column_name: &str) -> &mut ColumnDef {
        let columns = self
            .columns
            .entry(table_name.to_string())
            .or_insert_with(HashMap::new);
        columns
            .entry(column_name.to_string())
            .or_insert_with(|| ColumnDef::new(Alias::new(column_name)))
    }

    pub fn add_table_column(&mut self, table_name: &str, column_name: &str) -> &mut Self {
        let mut def = self.column(table_name, column_name).to_owned();
        self.table(table_name).col(&mut def).if_not_exists();
        self
    }

    pub fn finalize_columns(&mut self) -> &mut Self {
        for (table_name, column_defs) in self.columns.iter_mut() {
            let mut statement = self.tables
            .entry(table_name.to_string())
            .or_insert_with(|| {
                Table::create()
                    .table(Alias::new(table_name))
                    .if_not_exists()
                    .to_owned()
            });
            for (_col_name, col_def) in column_defs.iter_mut() {
                statement = statement.col(col_def);
            }
        }
        self.columns.clear();
        self
    }
}

impl Default for DatabaseBuilder {
    fn default() -> Self {
        DatabaseBuilder::new()
    }
}

#[test]
fn test_db_builder() {
    let mut builder = DatabaseBuilder::new();
    let table_name = "FooMsg";
    builder.column(table_name, "foo").string();
    builder.column(table_name, "bar").string();
    builder.finalize_columns();
    println!("{:#?}", builder);
}

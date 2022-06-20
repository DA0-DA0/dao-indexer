use convert_case::{Case, Casing};
use sea_orm::sea_query::{
    Alias, ColumnDef, /* ForeignKey, ForeignKeyAction,*/ Table, TableCreateStatement, PostgresQueryBuilder,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::collections::HashMap;

pub fn db_table_name(input_name: &str) -> String {
    input_name.to_case(Case::Snake)
}

pub fn db_column_name(input_name: &str) -> String {
    input_name.to_case(Case::Snake)
}

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
                    .table(Alias::new(&db_table_name(table_name)))
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
            .or_insert_with(|| ColumnDef::new(Alias::new(&db_column_name(column_name))))
    }

    pub fn add_table_column(&mut self, table_name: &str, column_name: &str) -> &mut Self {
        let mut def = self.column(table_name, column_name).to_owned();
        self.table(table_name).col(&mut def).if_not_exists();
        self
    }

    pub fn finalize_columns(&mut self) -> &mut Self {
        for (table_name, column_defs) in self.columns.iter_mut() {
            let mut statement = self
                .tables
                .entry(table_name.to_string())
                .or_insert_with(|| {
                    Table::create()
                        .table(Alias::new(&db_table_name(table_name)))
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

    pub async fn create_tables(&self, seaql_db: &DatabaseConnection) -> anyhow::Result<()> {
        let builder = seaql_db.get_database_backend();
        for (_table_name, table_def) in self.tables.iter() {
            let statement = builder.build(table_def);
            seaql_db.execute(statement).await?;
        }
        Ok(())
    }

    pub fn sql_string(&self) -> String {
        let mut statements = vec![];
        for (_table_name, table_def) in self.tables.iter() {
            let sql = table_def.to_string(PostgresQueryBuilder);
            statements.push(sql);
        }
        statements.join(";\n")
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

use convert_case::{Case, Casing};
use sea_orm::sea_query::{
    Alias, ColumnDef, ForeignKeyCreateStatement, PostgresQueryBuilder,
    /* ForeignKey, ForeignKeyAction,*/ Table, TableCreateStatement,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::collections::{BTreeMap, HashMap, HashSet};

use super::db_mapper::DatabaseMapper;

pub fn db_table_name(input_name: &str) -> String {
    input_name.to_case(Case::Snake)
}

pub fn db_column_name(input_name: &str) -> String {
    input_name.to_case(Case::Snake)
}

#[derive(Debug)]
pub struct DatabaseBuilder {
    tables: BTreeMap<String, TableCreateStatement>,
    table_constraints_filter: BTreeMap<String, HashSet<String>>,
    table_constraints: BTreeMap<String, Vec<ForeignKeyCreateStatement>>,
    columns: BTreeMap<String, HashMap<String, ColumnDef>>,
    pub value_mapper: DatabaseMapper,
}

impl DatabaseBuilder {
    pub fn new() -> Self {
        DatabaseBuilder {
            tables: BTreeMap::new(),
            table_constraints_filter: BTreeMap::new(),
            table_constraints: BTreeMap::new(),
            columns: BTreeMap::new(),
            value_mapper: DatabaseMapper::new(),
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
        self.value_mapper
            .add_mapping(
                table_name.to_string(),
                column_name.to_string(),
                table_name.to_string(),
                column_name.to_string(),
            )
            .unwrap();
        columns
            .entry(column_name.to_string())
            .or_insert_with(|| ColumnDef::new(Alias::new(&db_column_name(column_name))))
    }

    pub fn add_table_column(&mut self, table_name: &str, column_name: &str) -> &mut Self {
        let mut def = self.column(table_name, column_name).to_owned();
        self.table(table_name).col(&mut def).if_not_exists();
        self
    }

    /// Adds a database relationship between a field on one table and a different
    /// table. Defaults to using "fieldname_id" on one table and "id" on the other.
    pub fn add_relation(
        &mut self,
        source_table_name: &str,
        source_property_name: &str,
        destination_table_name: &str,
    ) -> anyhow::Result<()> {
        self.value_mapper.add_relational_mapping(
            source_table_name,
            source_property_name,
            destination_table_name,
            source_property_name,
        )?;
        let foreign_key = format!("{}_id", source_property_name);

        self.column(source_table_name, &foreign_key).integer();
        self.column(destination_table_name, "id")
            .unique_key()
            .integer();

        let db_source_table_name = db_table_name(source_table_name);
        let db_destination_table_name = db_table_name(destination_table_name);
        let foreign_key_create = ForeignKeyCreateStatement::new()
            .name(&foreign_key)
            .from_tbl(Alias::new(&db_source_table_name))
            .from_col(Alias::new(&foreign_key))
            .to_tbl(Alias::new(&db_destination_table_name))
            .to_col(Alias::new("id"))
            .to_owned();

        let fk_key = foreign_key_create.to_string(PostgresQueryBuilder);
        let constraints_set = self
            .table_constraints_filter
            .entry(destination_table_name.to_string())
            .or_insert_with(HashSet::new);
        if !constraints_set.contains(&fk_key) {
            constraints_set.insert(fk_key);
            let constraints = self
                .table_constraints
                .entry(destination_table_name.to_string())
                .or_insert(vec![]);
            constraints.push(foreign_key_create);
        }

        Ok(())
    }

    /// After all the schemas have added themselves to the various definitions,
    /// build the final table definitions and clear the processed column definitions.
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

    /// Use the final table definitions to physically build the database.
    pub async fn create_tables(&self, seaql_db: &DatabaseConnection) -> anyhow::Result<()> {
        if !self.columns.is_empty() {
            return Err(anyhow::anyhow!(
                "Builder not finalized. Please call `finalize_columns` before `create_tables`"
            ));
        }
        let builder = seaql_db.get_database_backend();
        for (table_name, table_def) in self.tables.iter() {
            let statement = builder.build(table_def);
            println!("Executing {}\n{:#?}", table_name, statement);
            seaql_db.execute(statement).await?;
        }
        // Now that all the tables are created, we can add the rest of the fields and constraints
        for (table_name, constraints) in self.table_constraints.iter() {
            for create_statement in constraints.iter() {
                // alter the table to add constraints
                let statement = builder.build(create_statement);
                println!(
                    "Executing foreign key statement {}\n{:#?}",
                    table_name, statement
                );
                seaql_db.execute(statement).await?;
            }
        }
        Ok(())
    }

    /// Human-readable SQL string for all definitions in this builder.
    pub fn sql_string(&self) -> anyhow::Result<String> {
        if !self.columns.is_empty() {
            return Err(anyhow::anyhow!(
                "Builder not finalized. Please call `finalize_columns` before `sql_string`"
            ));
        }
        let mut statements = vec![];
        for (_table_name, table_def) in self.tables.iter() {
            let sql = table_def.to_string(PostgresQueryBuilder);
            statements.push(sql);
        }
        Ok(statements.join(";\n"))
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

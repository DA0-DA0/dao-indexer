use log::debug;
use sea_orm::sea_query::{
    Alias, ColumnDef, ForeignKeyCreateStatement, PostgresQueryBuilder, Table, TableCreateStatement,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::collections::{BTreeMap, HashMap, HashSet};

use super::db_mapper::DatabaseMapper;
use super::db_util::{
    db_column_name, db_table_name, foreign_key, DEFAULT_ID_COLUMN_NAME,
    DEFAULT_TABLE_NAME_COLUMN_NAME, TARGET_ID_COLUMN_NAME
};

#[derive(Debug)]
pub struct DatabaseBuilder {
    tables: BTreeMap<String, TableCreateStatement>,
    table_constraints_filter: BTreeMap<String, HashSet<String>>,
    table_constraints: BTreeMap<String, Vec<ForeignKeyCreateStatement>>,
    columns: BTreeMap<String, HashMap<String, ColumnDef>>,
    pub value_mapper: DatabaseMapper,
    unique_key_map: HashSet<String>,
}

impl DatabaseBuilder {
    pub fn new() -> Self {
        DatabaseBuilder {
            tables: BTreeMap::new(),
            table_constraints_filter: BTreeMap::new(),
            table_constraints: BTreeMap::new(),
            columns: BTreeMap::new(),
            value_mapper: DatabaseMapper::new(),
            unique_key_map: HashSet::new(),
        }
    }
    pub fn table(&mut self, table_name: &str) -> &mut TableCreateStatement {
        self.tables
            .entry(table_name.to_string())
            .or_insert_with(|| {
                let sql_table_name = db_table_name(table_name);
                Table::create()
                    .table(Alias::new(&sql_table_name))
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
            .unwrap(); // TODO(gavin.doughtie): should not unwrap
        columns
            .entry(column_name.to_string())
            .or_insert_with(|| ColumnDef::new(Alias::new(&db_column_name(column_name))))
    }

    pub fn many_many(&mut self, table_name: &str, column_name: &str) {
        let join_table = format!("{}_{}", table_name, column_name);
        let l_key = foreign_key(table_name);
        let r_key = foreign_key(column_name);
        self.column(&join_table, &l_key);
        self.column(&join_table, &r_key);
    }

    pub fn add_table_column(&mut self, table_name: &str, column_name: &str) -> &mut Self {
        let mut def = self.column(table_name, column_name).to_owned();
        self.table(table_name).col(&mut def).if_not_exists();
        self
    }

    /// Adds a database relationship between a field on one table and a different
    /// table. Defaults to using "fieldname_id" on one table and DEFAULT_ID_COLUMN_NAME
    /// ("id") on the other.
    pub fn add_relation(
        &mut self,
        source_table_name: &str,
        source_property_name: &str,
        destination_table_name: &str,
    ) -> anyhow::Result<()> {
        if !self.unique_key_map.contains(destination_table_name) {
            self.unique_key_map
                .insert(destination_table_name.to_string());
            self.column(destination_table_name, DEFAULT_ID_COLUMN_NAME)
                .unique_key()
                .auto_increment()
                .integer();
        }
        if source_table_name == destination_table_name {
            debug!(
                "Not adding relation from {} to {}",
                source_table_name, destination_table_name
            );
            return Ok(());
        }
        self.value_mapper.add_relational_mapping(
            source_table_name,
            source_property_name,
            destination_table_name,
            DEFAULT_ID_COLUMN_NAME,
        )?;
        let fk = foreign_key(source_property_name);
        self.column(source_table_name, &fk).integer();

        let db_source_table_name = db_table_name(source_table_name);
        let db_destination_table_name = db_table_name(destination_table_name);
        let foreign_key_create = ForeignKeyCreateStatement::new()
            .name(&fk)
            .from_tbl(Alias::new(&db_source_table_name))
            .from_col(Alias::new(&fk))
            .to_tbl(Alias::new(&db_destination_table_name))
            .to_col(Alias::new(DEFAULT_ID_COLUMN_NAME))
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

    pub fn add_sub_message_relation(
        &mut self,
        source_table_name: &str,
        destination_table_name: &str,
    ) -> anyhow::Result<()> {
        // make sure source_table_name has an ID
        if !self.unique_key_map.contains(source_table_name) {
            self.unique_key_map.insert(source_table_name.to_string());
            self.column(source_table_name, DEFAULT_ID_COLUMN_NAME)
                .unique_key()
                .auto_increment()
                .integer();
        }

        // Adds a column in the sub-message table to point to
        // the sub-type record table by its name:
        self.column(source_table_name, DEFAULT_TABLE_NAME_COLUMN_NAME)
            .text();

        // add a sub message mapping BACK from sub-type record to sub-message
        self.add_relation(destination_table_name, source_table_name, source_table_name)?;

        // forward mapping from sub-message to specific sub-type table
        self.value_mapper.add_relational_mapping(
            source_table_name,
            TARGET_ID_COLUMN_NAME,
            destination_table_name,
            DEFAULT_ID_COLUMN_NAME,
        )
    }

    /// After all the schemas have added themselves to the various definitions,
    /// build the final table definitions and clear the processed column definitions.
    pub fn finalize_columns(&mut self) -> &mut Self {
        for (table_name, column_defs) in self.columns.iter_mut() {
            let sql_table_name = db_table_name(table_name);
            debug!(
                "finalize_columns for {}, db_name: {}",
                table_name, &sql_table_name
            );
            let mut statement = self
                .tables
                .entry(table_name.to_string())
                .or_insert_with(|| {
                    Table::create()
                        .table(Alias::new(&sql_table_name))
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
        for (_table_name, table_def) in self.tables.iter() {
            let statement = builder.build(table_def);
            // let statement_txt = format!("Executing {}\n{:#?}", table_name, statement);

            seaql_db.execute(statement).await?;
        }
        // Now that all the tables are created, we can add the rest of the fields and constraints
        for (table_name, constraints) in self.table_constraints.iter() {
            for create_statement in constraints.iter() {
                // alter the table to add constraints
                let statement = builder.build(create_statement);
                debug!(
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

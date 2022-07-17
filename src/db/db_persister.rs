use super::persister::Persister;
use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;
use log::debug;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Alias, Expr, IntoIden, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, JsonValue, Value};
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use super::db_util::{db_column_name, db_table_name, DEFAULT_ID_COLUMN_NAME};

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum Datatype {
    #[sea_orm(string_value = "Int")]
    Int,
    #[sea_orm(string_value = "BigInt")]
    BigInt,
    #[sea_orm(string_value = "String")]
    String,
}

impl Datatype {
    pub fn value_with_datatype(&self, value: Option<&JsonValue>) -> Value {
        match self {
            Datatype::Int => {
                if let Some(value) = value {
                    value.as_i64().into()
                } else {
                    None::<i64>.into()
                }
            }
            Datatype::BigInt => {
                if let Some(value) = value {
                    value.as_i64().into()
                } else {
                    None::<i64>.into()
                }
            }
            Datatype::String => {
                if let Some(value) = value {
                    value.as_str().into()
                } else {
                    None::<String>.into()
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct DatabasePersister {
    pub db: DatabaseConnection,
}

impl DatabasePersister {
    pub fn new(db: DatabaseConnection) -> Self {
        DatabasePersister { db }
    }
}

#[async_trait]
impl Persister<u64> for DatabasePersister {
    async fn save<'a>(
        &'a mut self,
        table_name: &'a str,
        column_names: &'a [&'a str],
        values: &'a [&'a JsonValue],
        id: &'a Option<u64>,
    ) -> Result<u64> {
        debug!(
            "saving table_name:{}, column_names:{:#?}, values:{:#?}, id:{:?}, db:{:?}",
            table_name, column_names, values, id, self.db
        );
        let mut update = false;
        let mut cols = vec![];
        let mut insert_columns = vec![];
        let mut vals = vec![];
        if id.is_some() {
            update = true;
        }
        for (value_index, column_name) in column_names.iter().enumerate() {
            let input_val = values[value_index];
            let val = match input_val {
                JsonValue::String(_v) => Datatype::String.value_with_datatype(Some(input_val)),
                JsonValue::Number(_v) => Datatype::BigInt.value_with_datatype(Some(input_val)),
                _ => Datatype::String.value_with_datatype(Some(input_val)),
            };
            let column_ident = Alias::new(&db_column_name(column_name)).into_iden();
            if update {
                cols.push((column_ident, val));
            } else {
                vals.push(val);
                insert_columns.push(column_ident);
            }
        }

        let builder = self.db.get_database_backend();

        if update {
            let stmt = Query::update()
                .table(Alias::new(&db_table_name(table_name)))
                .values(cols)
                .and_where(Expr::col(Alias::new(DEFAULT_ID_COLUMN_NAME).into_iden()).eq::<u64>(id.unwrap()))
                .to_owned();

            let result = self.db.execute(builder.build(&stmt)).await?;
            Ok(result.last_insert_id())
        } else {
            let stmt = Query::insert()
                .into_table(Alias::new(&db_table_name(table_name)))
                .columns(insert_columns)
                .values(vals)?
                .to_owned();
            let result = self.db.execute(builder.build(&stmt)).await?;
            Ok(result.last_insert_id())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult, Transaction};
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_persistence() -> anyhow::Result<()> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 1,
                },
                MockExecResult {
                    last_insert_id: 16,
                    rows_affected: 1,
                },
            ])
            .into_connection();
        let mut persister = DatabasePersister::new(db);
        let values: &[&serde_json::Value] = &[&json!("Gavin"), &json!("Doughtie"), &json!(1990)];
        let _id = persister
            .save(
                "Contact",
                &["first_name", "last_name", "birth_year"],
                values,
                &None,
            )
            .await?;
        // let id = Some(id);
        let log = persister.db.into_transaction_log();
        let expected_log = vec![Transaction::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"INSERT INTO "Contact" ("first_name", "last_name", "birth_year") VALUES ($1, $2, $3)"#,
            vec!["Gavin".into(), "Doughtie".into(), 1990_i64.into()],
        )];
        assert_eq!(expected_log, log);
        Ok(())
    }
}

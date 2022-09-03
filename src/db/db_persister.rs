use super::db_util::{db_column_name, db_table_name, DEFAULT_ID_COLUMN_NAME};
use super::persister::Persister;
use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;
use log::debug;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Alias, Expr, IntoIden, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, MockDatabaseConnection, JsonValue, Value};
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
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

pub type DbRef = Arc<RwLock<Box<DatabaseConnection>>>;

pub type DbRefMock = Arc<RwLock<Box<MockDatabaseConnection>>>;

pub fn make_db_ref(db: Box<DatabaseConnection>) -> DbRef {
    Arc::new(RwLock::new(db))
}

pub fn make_db_ref_mock(db: Box<MockDatabaseConnection>) -> DbRefMock {
    Arc::new(RwLock::new(db))
}

#[derive(Debug)]
pub struct DatabasePersister {
    pub db: DbRef,
    pub mock_db: Option<DbRefMock>,
}

impl DatabasePersister {
    pub fn new(db: DbRef) -> Self {
        DatabasePersister { db, mock_db: None }
    }

    pub fn with_mock_db(mock_db: DbRefMock) -> Self {
        let db: DatabaseConnection = DatabaseConnection::default();
        let db = make_db_ref(Box::new(db));
        DatabasePersister { db, mock_db: Some(mock_db) }
    }
}

#[async_trait]
impl Persister for DatabasePersister {
    type Id = u64;
    async fn save<'a>(
        &'a self,
        table_name: &'a str,
        column_names: &'a [&'a str],
        values: &'a [&'a JsonValue],
        id: Option<Self::Id>,
    ) -> Result<Self::Id> {
        let db = &self.db;
        debug!(
            "saving table_name:{}, column_names:{:#?}, values:{:#?}, id:{:?}, db:{:?}",
            table_name, column_names, values, id, db
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

        let persister_db = db.read().await;
        let builder = persister_db.get_database_backend();

        if update {
            let stmt = Query::update()
                .table(Alias::new(&db_table_name(table_name)))
                .values(cols)
                .and_where(
                    Expr::col(Alias::new(DEFAULT_ID_COLUMN_NAME).into_iden())
                        .eq::<u64>(id.unwrap() as u64),
                )
                .to_owned();

            let result = persister_db.execute(builder.build(&stmt)).await?;
            Ok(result.last_insert_id())
        } else {
            let stmt = Query::insert()
                .into_table(Alias::new(&db_table_name(table_name)))
                .columns(insert_columns)
                .values(vals)?
                .to_owned();
            let result = persister_db.execute(builder.build(&stmt)).await?;
            Ok(result.last_insert_id() as u64)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_persistence() -> anyhow::Result<()> {
        let db = MockDatabase::new(DatabaseBackend::Postgres).append_exec_results(vec![
            MockExecResult {
                last_insert_id: 15,
                rows_affected: 1,
            },
            MockExecResult {
                last_insert_id: 16,
                rows_affected: 1,
            },
        ]).into_connection();
        let db_ref = make_db_ref(Box::new(db));
        let persister = DatabasePersister::new(db_ref.clone());
        let values: &[&serde_json::Value] = &[&json!("Gavin"), &json!("Doughtie"), &json!(1990)];
        let id: u64 = persister
            .save(
                "Contact",
                &["first_name", "last_name", "birth_year"],
                values,
                None,
            )
            .await
            .unwrap();
        assert_eq!(15, id);
        // let log = db_ref.write().await.into_transaction_log();
        // let expected_log = vec![Transaction::from_sql_and_values(
        //     DatabaseBackend::Postgres,
        //     r#"INSERT INTO "contact" ("first_name", "last_name", "birth_year") VALUES ($1, $2, $3)"#,
        //     vec!["Gavin".into(), "Doughtie".into(), 1990_i64.into()],
        // )];
        // assert_eq!(expected_log, log);
        Ok(())
    }
}

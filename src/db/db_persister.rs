use super::db_util::{db_column_name, db_table_name, DEFAULT_ID_COLUMN_NAME};
use super::persister::Persister;
use anyhow::Result;
use async_trait::async_trait;
use tendermint::abci::Data;
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

// pub type ConnectionFunction<'a> = fn() -> &'a DatabaseConnection;


#[derive(Debug)]
pub enum ConnectionRef<'a, F> where F: Fn() -> &'a DatabaseConnection {
    Connection(DatabaseConnection),
    ConnectionFn(F)
}

impl<'a, F> ConnectionRef<'a, F> where F: Fn() -> &'a DatabaseConnection {
    pub fn to_ref(&'a self) -> &'a DatabaseConnection {
        match self {
            ConnectionRef::Connection(dbc) => dbc,
            ConnectionRef::ConnectionFn(dfn) => dfn()
        }
    }
}

pub fn make_db_ref(db: Box<DatabaseConnection>) -> DbRef {
    Arc::new(RwLock::new(db))
}


pub struct DatabasePersister<'a, F> where F: Fn() -> &'a DatabaseConnection {
    pub db: ConnectionRef<'a, F>
}

impl<'a, F> DatabasePersister<'a, F> where F: Fn() -> &'a DatabaseConnection {
    pub fn new(db: ConnectionRef<'a, F>) -> Self {
        DatabasePersister { db }
    }
}

impl<'a, F> Debug for DatabasePersister<'a, F> where F: Send + Sync + Debug + Fn() -> &'a DatabaseConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let owns_connection = match &self.db {
            ConnectionRef::Connection(_db) => true,
            _ => false
        };
        write!(f, "DatabasePersister owns_connection: {}", owns_connection)
    }
}

#[async_trait]
impl<'a, F> Persister<'a> for DatabasePersister<'a, F> where F: Send + Sync + Debug + Fn() -> &'a DatabaseConnection {
    type Id = u64;
    async fn save(
        &self,
        table_name: &str,
        column_names: &[&str],
        values: &[&JsonValue],
        id: Option<Self::Id>,
    ) -> Result<Self::Id> {
        let db = self.db.to_ref();
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

        let builder = db.get_database_backend();

        if update {
            let stmt = Query::update()
                .table(Alias::new(&db_table_name(table_name)))
                .values(cols)
                .and_where(
                    Expr::col(Alias::new(DEFAULT_ID_COLUMN_NAME).into_iden())
                        .eq::<u64>(id.unwrap() as u64),
                )
                .to_owned();

            let result = db.execute(builder.build(&stmt)).await?;
            Ok(result.last_insert_id())
        } else {
            let stmt = Query::insert()
                .into_table(Alias::new(&db_table_name(table_name)))
                .columns(insert_columns)
                .values(vals)?
                .to_owned();
            let result = db.execute(builder.build(&stmt)).await?;
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
        let get_ref = | | -> &DatabaseConnection {&db};
        let persister = DatabasePersister::new(ConnectionRef::ConnectionFn(get_ref));
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

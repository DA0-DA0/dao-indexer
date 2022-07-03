use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Alias, Expr, IntoIden, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, JsonValue, Value};
// use sea_orm::sea_query::{Alias, Cond, Expr, Iden, IntoIden, OnConflict, Query};
// use sea_orm::{
//     ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, FromQueryResult,
//     JoinType, JsonValue, QueryFilter, Value,
// };
use super::persister::Persister;
use serde::{Deserialize, Serialize};
use std::iter::Iterator;

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum Datatype {
    #[sea_orm(string_value = "Int")]
    Int,
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
        column_names: &'a [&'a String],
        values: &'a [&'a serde_json::Value],
        id: &'a Option<u64>,
    ) -> Result<u64> {
        debug!(
            "saving table_name:{}, column_names:{:#?}, values:{:#?}, id:{:?}, db:{:?}",
            table_name, column_names, values, id, self.db
        );
        let string_data_type = Datatype::String;
        let mut update = false;
        let mut cols = vec![];
        let mut insert_columns = vec![];
        let mut vals = vec![];
        if id.is_some() {
            update = true;
        }
        for (value_index, column_name) in column_names.iter().enumerate() {
            let val = values[value_index];
            let val = string_data_type.value_with_datatype(Some(val));
            let column_ident = Alias::new(column_name).into_iden();
            if update {
                cols.push((column_ident, val));
            } else {
                vals.push(val);
                insert_columns.push(column_ident);
            }
        }
        // let mut vals = vec![];
        // for value in values {
        //     let val = string_data_type.value_with_datatype(Some(value));
        //     vals.push(val.clone());
        // }
        let builder = self.db.get_database_backend();

        if update {
            let stmt = Query::update()
                .table(Alias::new(table_name))
                .values(cols)
                // .values::<Value>(vals.iter().collect_tuple().unwrap())
                .and_where(Expr::col(Alias::new("id").into_iden()).eq::<i64>(id.unwrap() as i64))
                .to_owned();

            let result = self.db.execute(builder.build(&stmt)).await?;
            println!("{:#?}", result);
            Ok(result.last_insert_id())
            // stmt.values_panic(vals);
        } else {
            let stmt = Query::insert()
                .into_table(Alias::new(table_name))
                .columns(insert_columns)
                .values(vals)?
                .to_owned();
            let result = self.db.execute(builder.build(&stmt)).await?;
            println!("{:#?}", result);
            Ok(result.last_insert_id())
        }
        // if upsert {
        //     stmt.on_conflict(
        //         OnConflict::column(NodeIden::Name)
        //             .update_columns(cols)
        //             .to_owned(),
        //     );
        // }

        // for node_json in node_json_batch.nodes.into_iter() {
        //     let mut vals = vec![node_json.name.as_str().into()];
        //     for attribute in attributes.iter() {
        //         let name = &attribute.name;
        //         let val = attribute
        //             .datatype
        //             .value_with_datatype(node_json.attributes.get(name));
        //         vals.push(val);
        //     }
        //     stmt.values_panic(vals);
        // }

        // Ok(0)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
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
        let id = persister
            .save(
                "Contact",
                &[&"first_name".to_string(), &"last_name".to_string()],
                &[&json!("Gavin"), &json!("Doughtie")],
                &None,
            )
            .await?;
        let id = Some(id);
        let log = persister.db.into_transaction_log();
        let log_msg = format!("{:?} log:\n{:#?}", id, log);
        println!("{}", log_msg);
        Ok(())
    }
}

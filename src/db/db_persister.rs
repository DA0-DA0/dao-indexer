use super::persister::Persister;
use anyhow::Result;
use log::debug;
use serde_json;
use sea_orm::entity::prelude::*;
use sea_orm::{
  ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, FromQueryResult, JoinType, JsonValue,
  QueryFilter, Value,
};
use sea_orm::sea_query::{Alias, Cond, Iden, Expr, IntoIden, OnConflict, Query};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum Datatype {
    #[sea_orm(string_value = "Int")]
    Int,
    #[sea_orm(string_value = "String")]
    String,
}

impl Datatype {
    pub fn value_with_datatype(&self, value: Option<&serde_json::Value>) -> Value {
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
    db: DatabaseConnection,
}

impl DatabasePersister {
    pub fn new(db: DatabaseConnection) -> Self {
        DatabasePersister { db }
    }
}

#[async_trait]
impl Persister for DatabasePersister {
    async fn save(
        &mut self,
        table_name: &str,
        column_name: &str,
        value: &serde_json::Value,
        id: &Option<usize>,
    ) -> Result<usize> {
        debug!(
            "saving table_name:{}, column_name:{}, value:{}, id:{:?}, db:{:?}",
            table_name, column_name, value, id, self.db
        );
        let mut cols = vec![Alias::new(column_name).into_iden()];

        let mut stmt = Query::insert();
        stmt.into_table(Alias::new(table_name))
            .columns(cols.clone());
        let data_type = Datatype::String;
        let val = data_type.value_with_datatype(Some(value));
        let vals = vec![val];
        stmt.values_panic(vals);
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

        let builder = self.db.get_database_backend();
        self.db.execute(builder.build(&stmt)).await?;
        Ok(0)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use sea_orm::{entity::prelude::*, DatabaseBackend, MockDatabase};
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_persistence() -> anyhow::Result<()> {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let mut persister = DatabasePersister::new(db);
        let id = persister
            .save("Contact", "first_name", &json!("Gavin"), &None)
            .await?;
        let id = Some(id);
        let result = persister
            .save("Contact", "last_name", &json!("Doughtie"), &id)
            .await;
        Ok(())
    }
}

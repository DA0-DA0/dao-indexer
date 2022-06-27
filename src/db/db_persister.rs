use super::persister::Persister;
use anyhow::Result;
use log::debug;
use sea_orm::DatabaseConnection;
use serde_json::Value;

#[derive(Debug)]
pub struct DatabasePersister {
    db: DatabaseConnection,
}

impl DatabasePersister {
    pub fn new(db: DatabaseConnection) -> Self {
        DatabasePersister { db }
    }
}

impl Persister for DatabasePersister {
    fn save(
        &mut self,
        table_name: &str,
        column_name: &str,
        value: &Value,
        id: &Option<usize>,
    ) -> Result<usize> {
        debug!(
            "saving table_name:{}, column_name:{}, value:{}, id:{:?}, db:{:?}",
            table_name, column_name, value, id, self.db
        );
        Ok(0)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use sea_orm::{entity::prelude::*, DatabaseBackend, MockDatabase};
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_persistence() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let mut persister = DatabasePersister::new(db);
        let id = persister
            .save("Contact", "first_name", &json!("Gavin"), &None)
            .unwrap();
        let id = Some(id);
        assert!(persister
            .save("Contact", "last_name", &json!("Doughtie"), &id)
            .is_ok());
        Ok(())
    }
}

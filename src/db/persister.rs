use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::cell::RefCell;
use std::sync::Arc;
use async_std::sync::RwLock;
use sea_orm::DatabaseConnection;

pub type PersistColumnNames<'a> = &'a [&'a str];
pub type PersistValues<'a> = &'a [&'a Value];

/// Trait for persisting a message.
#[async_trait]
pub trait Persister<'a>: Send + Sync + std::fmt::Debug {
    type Id;
    async fn save(
        &self,
        table_name: &str,
        column_names: & [&str],
        values: & [&Value],
        id: Option<Self::Id>,
    ) -> Result<Self::Id>;

    fn get_db(&mut self) -> Option<&DatabaseConnection> {
        None
    }
}

pub type PersisterRef<'a, T> = Arc<RwLock<RefCell<Box<dyn Persister<'a, Id = T>>>>>;

pub fn make_persister_ref<'a, T>(persister: Box<dyn Persister<'a, Id = T>>) -> PersisterRef<'a, T> {
    Arc::new(RwLock::from(RefCell::from(persister)))
}

#[derive(Debug)]
pub struct StubPersister {}

#[async_trait]
impl<'a> Persister<'a> for StubPersister {
    type Id = u64;
    async fn save(
        &self,
        _table_name: &str,
        _column_names: &[&str],
        _values: &[& Value],
        _id: Option<Self::Id>,
    ) -> Result<Self::Id> {
        Ok(0)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::RwLock;
    use tokio::test;

    type Record = BTreeMap<String, Value>;
    #[derive(Debug)]
    pub struct TestPersister {
        pub tables: RwLock<BTreeMap<String, HashMap<usize, Record>>>,
    }

    impl TestPersister {
        #[allow(dead_code)]
        pub fn new() -> Self {
            let tables: BTreeMap<String, HashMap<usize, Record>> = BTreeMap::new();
            let tables = RwLock::from(tables);
            TestPersister { tables }
        }
    }

    impl Default for TestPersister {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl<'a> Persister<'a> for TestPersister {
        type Id = u64;
        async fn save(
            &self,
            table_name: &str,
            column_names: &[&str],
            values: &[&Value],
            id: Option<Self::Id>,
        ) -> Result<Self::Id> {
            let mut tables = self
                .tables
                .write()
                .expect("Failed to acquire a write lock on tables");
            let records = tables
                .entry(table_name.to_string())
                .or_insert_with(HashMap::new);
            let id = match id {
                Some(id) => id,
                _ => records.len() as u64,
            };

            let record = records.entry(id as usize).or_insert_with(BTreeMap::new);

            for (value_index, column_name) in column_names.iter().enumerate() {
                if let Some(value) = values.get(value_index) {
                    record.insert(column_name.to_string(), (**value).clone());
                }
            }
            Ok(id)
        }
    }

    #[test]
    async fn test_persister_trait() -> anyhow::Result<()> {
        let persister: TestPersister = TestPersister::new();
        let id = persister
            .save(
                "contacts",
                &["first_name", "last_name", "birth_year"],
                &[
                    &Value::String("Gavin".to_string()),
                    &Value::String("Doughtie".to_string()),
                    &serde_json::json!(1962u64),
                ],
                None,
            )
            .await
            .unwrap();

        println!("Persisted {}:\n{:#?}", id, persister);
        Ok(())
    }
}

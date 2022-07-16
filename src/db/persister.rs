use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub type PersistColumnNames<'a> = &'a [&'a str];
pub type PersistValues<'a> = &'a [&'a Value];

/// Trait for persisting a message.
#[async_trait]
pub trait Persister<T> : Send {
    async fn save<'a>(
        &'a mut self,
        table_name: &'a str,
        column_names: &'a [&'a str],
        values: &'a [&'a Value],
        id: &'a Option<T>
    ) -> Result<T>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use tokio::test;

    type Record = BTreeMap<String, Value>;
    #[derive(Debug)]
    pub struct TestPersister<T = usize> {
        pub tables: BTreeMap<String, HashMap<T, Record>>,
    }

    impl<T> TestPersister<T> {
        #[allow(dead_code)]
        pub fn new() -> Self {
            let mut tables: BTreeMap<String, HashMap<T, Record>> = BTreeMap::new();
            TestPersister {
                tables
            }
        }
    }

    impl Default for TestPersister {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl Persister<usize> for TestPersister {
        async fn save<'a>(
            &'a mut self,
            table_name: &'a str,
            column_names: &'a [&'a str],
            values: &'a [&'a Value],
            id: &'a Option<usize>,
        ) -> Result<usize> {
            let records: &mut HashMap<usize, Record> = self
                .tables
                .entry(table_name.to_string())
                .or_insert_with(HashMap::new);
            let id = match id {
                Some(id) => *id,
                _ => records.len(),
            };

            let record = records.entry(id).or_insert_with(BTreeMap::new);

            for (value_index, column_name) in column_names.iter().enumerate() {
                if let Some(value) = values.get(value_index) {
                    record.insert(column_name.to_string(), (**value).clone());
                }
            }
            Ok(id)
        }
    }

    #[async_trait]
    impl Persister<u64> for TestPersister {
        async fn save<'a>(
            &'a mut self,
            table_name: &'a str,
            column_names: &'a [&'a str],
            values: &'a [&'a Value],
            id: &'a Option<u64>,
        ) -> Result<u64> {
            let records = self
                .tables
                .entry(table_name.to_string())
                .or_insert_with(HashMap::new);
            let id = match id {
                Some(id) => *id,
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
        let mut persister: TestPersister<usize> = TestPersister::<usize>::new();
        let id: usize = persister
            .save(
                "contacts",
                &[
                    &"first_name".to_string(),
                    &"last_name".to_string(),
                    &"birth_year".to_string(),
                ],
                &[
                    &Value::String("Gavin".to_string()),
                    &Value::String("Doughtie".to_string()),
                    &serde_json::json!(1962u64),
                ],
                &None,
            )
            .await
            .unwrap();

        println!("Persisted {}:\n{:#?}", id, persister);
        Ok(())
    }
}

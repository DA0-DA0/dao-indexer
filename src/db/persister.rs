use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub type PersistColumnNames<'a> = &'a [&'a str];
pub type PersistValues<'a> = &'a [&'a Value];

/// Trait for persisting a message.
#[async_trait]
pub trait Persister: Send + Sync + std::fmt::Debug {
    type Id;
    async fn save<'a>(
        &'a mut self,
        table_name: &'a str,
        column_names: &'a [&'a str],
        values: &'a [&'a Value],
        id: Option<Self::Id>,
    ) -> Result<Self::Id>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use tokio::test;

    type Record = BTreeMap<String, Value>;
    #[derive(Debug)]
    pub struct TestPersister {
        pub tables: BTreeMap<String, HashMap<usize, Record>>,
    }

    impl TestPersister {
        #[allow(dead_code)]
        pub fn new() -> Self {
            let tables: BTreeMap<String, HashMap<usize, Record>> = BTreeMap::new();
            TestPersister { tables }
        }
    }

    impl Default for TestPersister {
        fn default() -> Self {
            Self::new()
        }
    }

    // #[async_trait]
    // impl Persister for TestPersister {
    //     async fn save<'a>(
    //         &'a mut self,
    //         table_name: &'a str,
    //         column_names: &'a [&'a str],
    //         values: &'a [&'a Value],
    //         id: Option<usize>,
    //     ) -> Result<usize> {
    //         let records: &mut HashMap<usize, Record> = self
    //             .tables
    //             .entry(table_name.to_string())
    //             .or_insert_with(HashMap::new);
    //         let id = match id {
    //             Some(id) => id,
    //             _ => records.len(),
    //         };

    //         let record = records.entry(id).or_insert_with(BTreeMap::new);

    //         for (value_index, column_name) in column_names.iter().enumerate() {
    //             if let Some(value) = values.get(value_index) {
    //                 record.insert(column_name.to_string(), (**value).clone());
    //             }
    //         }
    //         Ok(id)
    //     }
    // }

    #[async_trait]
    impl Persister for TestPersister {
        type Id = u64;
        async fn save<'a>(
            &'a mut self,
            table_name: &'a str,
            column_names: &'a [&'a str],
            values: &'a [&'a Value],
            id: Option<Self::Id>,
        ) -> Result<Self::Id> {
            let records = self
                .tables
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
        let mut persister: TestPersister = TestPersister::new();
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

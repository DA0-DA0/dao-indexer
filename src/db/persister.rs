use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

// pub type PersistColumnNames<'a> = dyn IntoIterator<Item = &'a str, IntoIter = dyn core::iter::Iterator<Item = &'a str>>;
// pub type PersistValues<'a> = dyn IntoIterator<Item = &'a Value, IntoIter = dyn core::iter::Iterator<Item = &'a Value>>;

pub type PersistColumnNames<'a> = &'a Vec<&'a str>;
pub type PersistValues<'a> = &'a Vec<Value>;

/// Trait for persisting a message.
/// T is the ID type.
#[async_trait]
pub trait Persister<T = u64> {
    async fn save<'a>(
        &mut self,
        table_name: &str,
        column_names: PersistColumnNames,
        values: PersistValues,
        id: &Option<T>,
    ) -> Result<T>;
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
            TestPersister {
                tables: BTreeMap::new(),
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
                &mut self,
                table_name: &str,
                column_names: PersistColumnNames,
                values: PersistValues,
                id: &Option<usize>,
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

            let values = values.iter();
            for column_name in column_names {
                if let Some(value) = values.next() {
                    record.insert(column_name.to_string(), value.clone());
                }
            }

            Ok(id)
        }
    }

    #[test]
    async fn test_persister_trait() -> anyhow::Result<()> {
        let mut persister = TestPersister::new();
        let id = persister
            .save(
                "contacts",
                vec!["first_name"].iter(),
                vec![&Value::String("Gavin".to_string())].iter(),
                &None,
            )
            .await
            .unwrap();
        persister
            .save(
                "contacts",
                "last_name",
                &Value::String("Doughtie".to_string()),
                &Some(id),
            )
            .await?;
        let year = serde_json::json!(1962u64);
        persister
            .save("contacts", "birth_year", &year, &Some(id))
            .await?;

        let id = persister
            .save(
                "contacts",
                "first_name",
                &Value::String("Kristina".to_string()),
                &None,
            )
            .await?;
        persister
            .save(
                "contacts",
                "last_name",
                &Value::String("Helwing".to_string()),
                &Some(id),
            )
            .await?;
        let year = serde_json::json!(1978);
        persister
            .save("contacts", "birth_year", &year, &Some(id))
            .await?;

        println!("Persisted:\n{:#?}", persister);
        Ok(())
    }
}

use anyhow::Result;
use serde_json::Value;

/// Trait for persisting a message.
/// T is the ID type.
pub trait Persister<T = usize> {
    fn save(
        &mut self,
        table_name: &str,
        column_name: &str,
        value: &Value,
        id: &Option<T>,
    ) -> Result<T>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};

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

    impl Persister<usize> for TestPersister {
        fn save(
            &mut self,
            table_name: &str,
            column_name: &str,
            value: &Value,
            id: &Option<usize>,
        ) -> anyhow::Result<usize> {
            let records: &mut HashMap<usize, Record> = self
                .tables
                .entry(table_name.to_string())
                .or_insert_with(HashMap::new);
            let id = match id {
                Some(id) => *id,
                _ => records.len(),
            };

            let record = records.entry(id).or_insert_with(BTreeMap::new);
            record.insert(column_name.to_string(), value.clone());

            Ok(id)
        }
    }

    #[test]
    fn test_persister_trait() -> anyhow::Result<()> {
        let mut persister = TestPersister::new();
        let id = persister
            .save(
                "contacts",
                "first_name",
                &Value::String("Gavin".to_string()),
                &None,
            )
            .unwrap();
        persister.save(
            "contacts",
            "last_name",
            &Value::String("Doughtie".to_string()),
            &Some(id),
        )?;
        let year = serde_json::json!(1962u64);
        persister.save("contacts", "birth_year", &year, &Some(id))?;

        let id = persister
            .save(
                "contacts",
                "first_name",
                &Value::String("Kristina".to_string()),
                &None,
            )
            .unwrap();
        persister.save(
            "contacts",
            "last_name",
            &Value::String("Helwing".to_string()),
            &Some(id),
        )?;
        let year = serde_json::json!(1978);
        persister.save("contacts", "birth_year", &year, &Some(id))?;

        println!("Persisted:\n{:#?}", persister);
        Ok(())
    }
}

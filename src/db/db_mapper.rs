use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// Relational mapping
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseRelationship {
    pub source_table: String,
    pub source_column: String,
    pub destination_table: String,
    pub destination_column: String,
    pub join_table: Option<String>,
}

impl DatabaseRelationship {
    pub fn new(
        source_table: &str,
        source_column: &str,
        destination_table: &str,
        destination_column: &str,
        join_table: Option<String>,
    ) -> DatabaseRelationship {
        DatabaseRelationship {
            source_table: source_table.to_string(),
            source_column: source_column.to_string(),
            destination_table: destination_table.to_string(),
            destination_column: destination_column.to_string(),
            join_table,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldMapping {
    pub message_name: String,
    pub field_name: String,
    pub table_name: String,
    pub column_name: String,
    // TODO(gavindoughtie): will probably need a policy defining HOW to put a value in a field
}

impl FieldMapping {
    pub fn new(message_name: &str, field_name: &str, table_name: &str, column_name: &str) -> Self {
        FieldMapping {
            message_name: message_name.to_string(),
            field_name: field_name.to_string(),
            table_name: table_name.to_string(),
            column_name: column_name.to_string(),
        }
    }
}

trait Persister<T> {
    fn save(&mut self, table_name: &str, column_name: &str, value: &Value, id: Option<T>) -> anyhow::Result<T>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseMapper {
    pub relationships: HashMap<String, DatabaseRelationship>,
    pub mappings: HashMap<String, FieldMapping>,
}

impl DatabaseMapper {
    pub fn new() -> DatabaseMapper {
        DatabaseMapper {
            relationships: HashMap::new(),
            mappings: HashMap::new(),
        }
    }

    // Add an inbound mapping FROM the message TO the database.
    pub fn add_mapping(
        &mut self,
        message_name: &str,
        field_name: &str,
        table_name: &str,
        column_name: &str,
    ) -> anyhow::Result<()> {
        debug!(
            "add_mapping(message_name: {}, field_name: {}, table_name: {}, column_name: {})",
            message_name, field_name, table_name, column_name
        );
        let mapping = FieldMapping::new(message_name, field_name, table_name, column_name);
        self.mappings.insert(message_name.to_string(), mapping);
        Ok(())
    }

    pub fn add_relational_mapping(
        &mut self,
        message_name: &str,
        field_name: &str,
        table_name: &str,
        column_name: &str,
    ) -> anyhow::Result<()> {
        println!("add_mapping(add_relational_mapping: {}, field_name: {}, table_name: {}, column_name: {})", message_name, field_name, table_name, column_name);
        let relation =
            DatabaseRelationship::new(message_name, field_name, table_name, column_name, None);
        self.relationships
            .insert(message_name.to_string(), relation);
        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        vec![]
    }

    pub fn persist_message(&mut self, table_name: &str, msg: &Value) -> anyhow::Result<()> {
        println!("persist_msg {}, {:#?}", table_name, msg);

        for key in self.keys() {
            if let Some(Value::String(val)) = msg.get(&key) {
                println!("Saving {}:{}={}", table_name, key, val);
            }
        }
        Ok(())
    }
}

impl Default for DatabaseMapper {
    fn default() -> Self {
        Self::new()
    }
}

type Record = BTreeMap<String, Value>;
struct TestPersister {
    pub tables: BTreeMap<String, HashMap<usize, Record>>,
}

impl TestPersister {
    pub fn new() -> Self {
        TestPersister {
            tables: BTreeMap::new(),
        }
    }
}

impl Persister<usize> for TestPersister {
    fn save(&mut self, table_name: &str, column_name: &str, value: &Value, id: Option<usize>) -> anyhow::Result<usize> {
        let records: &mut HashMap<usize, Record> = self.tables
            .entry(table_name.to_string())
            .or_insert_with(|| {
                HashMap::new()
            });            
        let id = match id {
            Some(id) => id,
            _ => records.len()
        };

        let record = records.entry(id).or_insert_with(|| BTreeMap::new());
        record.insert(column_name.to_string(), value.clone());

        Ok(id)
    }
}

#[test]
fn test_map_to_records() {
  let persister = TestPersister::new();
  persister.save("Table", "Column", Value::String::new("Test value"), None);
}
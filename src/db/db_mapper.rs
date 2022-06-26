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
    pub fn new(
        message_name: String,
        field_name: String,
        table_name: String,
        column_name: String,
    ) -> Self {
        FieldMapping {
            message_name,
            field_name,
            table_name,
            column_name,
        }
    }
}

pub trait Persister<T> {
    fn save(
        &mut self,
        table_name: &str,
        column_name: &str,
        value: &Value,
        id: &Option<T>,
    ) -> anyhow::Result<T>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseMapper {
    pub relationships: HashMap<String, DatabaseRelationship>,

    // Map from a message name to a dictionary of mappings
    pub mappings: HashMap<String, HashMap<String, FieldMapping>>,
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
        message_name: String,
        field_name: String,
        table_name: String,
        column_name: String,
    ) -> anyhow::Result<()> {
        debug!(
            "add_mapping(message_name: {}, field_name: {}, table_name: {}, column_name: {})",
            message_name, field_name, table_name, column_name
        );
        let message_mappings = self
            .mappings
            .entry(message_name.clone())
            .or_insert_with(HashMap::new);
        let mapping = FieldMapping::new(message_name, field_name, table_name, column_name.clone());
        message_mappings.insert(column_name, mapping);
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

    pub fn persist_message<T>(
        &mut self,
        persister: &mut dyn Persister<T>,
        table_name: &str,
        msg: &Value,
    ) -> anyhow::Result<Option<T>> {
        println!("persist_msg {}, {:#?}", table_name, msg);

        let mut record_id: Option<T> = None;

        let mapping = self.mappings.get(table_name);
        if mapping.is_none() {
            return Err(anyhow::anyhow!("no mapping for {}", table_name));
        }
        let mapping = mapping.unwrap();

        for field_name in mapping.keys() {
            if let Some(val) = msg.get(field_name) {
                println!("Saving {}:{}={}", table_name, field_name, val);
                if let Ok(updated_id) = persister.save(table_name, field_name, val, &record_id) {
                    if record_id.is_none() {
                        record_id = Some(updated_id);
                    }
                }
            }
        }
        Ok(record_id)
    }
}

impl Default for DatabaseMapper {
    fn default() -> Self {
        Self::new()
    }
}

type Record = BTreeMap<String, Value>;
#[derive(Debug)]
struct TestPersister {
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

#[test]
fn test_mapper_to_persistence() -> anyhow::Result<()> {
    let mut mapper = DatabaseMapper::new();
    let message_name = "Contact".to_string();
    let first_name_field_name = "first_name".to_string();
    let last_name_field_name = "last_name".to_string();
    let birth_year_field_name = "birth_year".to_string();
    mapper.add_mapping(
        message_name.clone(),
        first_name_field_name.clone(),
        message_name.clone(),
        first_name_field_name.clone(),
    )?;
    mapper.add_mapping(
        message_name.clone(),
        last_name_field_name.clone(),
        message_name.clone(),
        last_name_field_name.clone(),
    )?;
    mapper.add_mapping(
        message_name.clone(),
        birth_year_field_name.clone(),
        message_name.clone(),
        birth_year_field_name.clone(),
    )?;

    let record_one = serde_json::json!({
      "first_name": "Gavin",
      "last_name": "Doughtie",
      "birth_year": 1962u64
    });

    let record_two = serde_json::json!({
      "first_name": "Kristina",
      "last_name": "Helwing",
      "birth_year": 1978u64
    });

    let mut persister = TestPersister::new();
    let record_one_id = mapper
        .persist_message(&mut persister, &message_name, &record_one)
        .unwrap();
    let record_two_id = mapper
        .persist_message(&mut persister, &message_name, &record_two)
        .unwrap();

    let records_for_message = persister.tables.get(&message_name).unwrap();
    let persisted_record_one = records_for_message.get(&record_one_id.unwrap()).unwrap();
    let persisted_record_two = records_for_message.get(&record_two_id.unwrap()).unwrap();
    assert_eq!(
        record_one.get(first_name_field_name.clone()).unwrap(),
        persisted_record_one.get(&first_name_field_name).unwrap()
    );
    assert_eq!(
        record_one.get(last_name_field_name.clone()).unwrap(),
        persisted_record_one.get(&last_name_field_name).unwrap()
    );
    assert_eq!(
        record_one.get(birth_year_field_name.clone()).unwrap(),
        persisted_record_one.get(&birth_year_field_name).unwrap()
    );
    assert_eq!(
        record_two.get(first_name_field_name.clone()).unwrap(),
        persisted_record_two.get(&first_name_field_name).unwrap()
    );
    println!("persisted:\n{:#?}", persister);

    Ok(())
}

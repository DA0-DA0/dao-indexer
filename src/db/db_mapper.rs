use super::persister::Persister;
use async_recursion::async_recursion;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
// use uuid::Uuid;
// use core::fmt::Display;
// use futures::future::{BoxFuture, FutureExt};

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
    pub recursive: bool,
    pub related_table: String,
}

impl FieldMapping {
    pub fn new(
        message_name: String,
        field_name: String,
        table_name: String,
        column_name: String,
        recursive: bool,
        related_table: String,
    ) -> Self {
        FieldMapping {
            message_name,
            field_name,
            table_name,
            column_name,
            recursive,
            related_table,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseMapper {
    // Maps from message name to a map of field names to DB relationships
    pub relationships: HashMap<String, HashMap<String, DatabaseRelationship>>,

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
        let mapping = FieldMapping::new(
            message_name,
            field_name,
            table_name,
            column_name.clone(),
            false,
            "".to_string(),
        );
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
        let message_relationships = self
            .relationships
            .entry(message_name.to_string())
            .or_insert_with(HashMap::new);
        message_relationships.insert(field_name.to_string(), relation);
        Ok(())
    }

    #[async_recursion]
    pub async fn persist_message(
        &self,
        persister: &mut dyn Persister<u64>,
        table_name: &str,
        msg: &Value,
        record_id: Option<u64>,
    ) -> anyhow::Result<u64> {
        let mapping = self.mappings.get(table_name);
        if mapping.is_none() {
            return Err(anyhow::anyhow!("no mapping for {}", table_name));
        }
        let mapping = mapping.unwrap();

        // So the strategy here is to recursively go through the message
        // persisting the relational messages first and then the top-level
        // messages given the IDs from the persisted related messages.
        let mut columns: Vec<&str> = vec![];
        let mut values: Vec<&Value> = vec![];
        let mut child_id_columns: Vec<&str> = vec![];
        let mut child_id_values: Vec<Value> = vec![];
        let relationships = self.relationships.get(table_name);
        for (key, field_mapping) in &*mapping {
            if let Some(value) = msg.get(&field_mapping.field_name) {
                if field_mapping.recursive {
                    if let Some(relationships) = relationships {
                        if let Some(field_relationship) =
                            relationships.get(&field_mapping.field_name)
                        {
                            let child_id = self
                                .persist_message(
                                    persister,
                                    &field_mapping.related_table,
                                    value,
                                    None,
                                )
                                .await?;
                            let child_id_value = serde_json::json!(child_id);
                            child_id_columns.push(&field_relationship.destination_column);
                            child_id_values.push(child_id_value);
                        }
                    }
                } else {
                    columns.push(key);
                    values.push(value);
                }
            }
        }
        let mut db_id = 0;
        columns.append(&mut child_id_columns);
        for child_id_value in child_id_values.iter() {
            values.push(child_id_value);
        }
        if !columns.is_empty() {
            db_id = persister
                .save(table_name, &columns[..], &values[..], &record_id)
                .await?;
        }
        Ok(db_id)
    }
}

impl Default for DatabaseMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::persister::tests::*;
    use tokio::test;

    #[test]
    async fn test_mapper_to_persistence() -> anyhow::Result<()> {
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

        let mut persister = TestPersister::<u64>::new();
        let record_one_id: u64 = mapper
            .persist_message(&mut persister, &message_name, &record_one, None)
            .await?;
        let record_two_id: u64 = mapper
            .persist_message(&mut persister, &message_name, &record_two, None)
            .await?;

        let records_for_message = persister.tables.get(&message_name).unwrap();
        let persisted_record_one = records_for_message.get(&record_one_id).unwrap();
        let persisted_record_two = records_for_message.get(&record_two_id).unwrap();
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
}

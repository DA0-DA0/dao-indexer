use super::persister::Persister;
use async_recursion::async_recursion;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
        source_table: String,
        source_column: String,
        destination_table: String,
        destination_column: String,
        join_table: Option<String>,
    ) -> DatabaseRelationship {
        DatabaseRelationship {
            source_table,
            source_column,
            destination_table,
            destination_column,
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

    // Points from "MessageName.field_name to RelatedMessageName.related_column_name"
    pub fn add_relational_mapping(
        &mut self,
        message_name: &str,
        field_name: &str,
        related_message_name: &str,
        related_column_name: &str,
    ) -> anyhow::Result<()> {
        debug!("add_mapping(add_relational_mapping: {}, field_name: {}, table_name: {}, column_name: {})", message_name, field_name, related_message_name, related_column_name);
        let relation = DatabaseRelationship::new(
            message_name.to_string(),
            field_name.to_string(),
            related_message_name.to_string(),
            related_column_name.to_string(),
            None,
        );
        let message_relationships = self
            .relationships
            .entry(message_name.to_string())
            .or_insert_with(HashMap::new);
        message_relationships.insert(field_name.to_string(), relation);

        let message_mappings = self
            .mappings
            .entry(message_name.to_string())
            .or_insert_with(HashMap::new);

        let mapping = FieldMapping::new(
            message_name.to_string(),
            field_name.to_string(),
            related_message_name.to_string(),
            related_column_name.to_string(),
            true,
            related_message_name.to_string(),
        );
        message_mappings.insert(field_name.to_string(), mapping);
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
        let object_msg = match msg {
            Value::Object(msg) => Some(msg),
            _ => None,
        };
        if object_msg.is_none() {
            return Err(anyhow::anyhow!(
                "unable to persist non-object message {:#?}",
                msg
            ));
        }
        let msg = object_msg.unwrap();

        // So the strategy here is to recursively go through the message
        // persisting the relational messages first and then the top-level
        // messages given the IDs from the persisted related messages.
        let mut columns: Vec<&str> = vec![];
        let mut values: Vec<&Value> = vec![];
        let mut child_id_columns: Vec<&str> = vec![];
        let mut child_id_values: Vec<Value> = vec![];
        let relationships = self.relationships.get(table_name);

        for (key, value) in msg {
            if let Some(relationships) = relationships {
                if let Some(field_relationship) = relationships.get(key) {
                    println!("field_relationship: {:#?}", field_relationship);
                    if let Some(field_mapping) = mapping.get(key) {
                        println!("field_mapping: {:#?}", field_mapping);
                        let child_id = self
                            .persist_message(persister, &field_mapping.related_table, value, None)
                            .await?;
                        let child_id_value = serde_json::json!(child_id);
                        child_id_columns.push(&field_relationship.destination_column);
                        child_id_values.push(child_id_value);
                    }
                }
            }
            if let Some(field_mapping) = mapping.get(key) {
                debug!("persisting {:#?} {}={:#?}", field_mapping, key, value);
                columns.push(key);
                values.push(value);
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
    use crate::db::db_persister::DatabasePersister;
    use crate::db::persister::tests::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use tokio::test;

    #[test]
    async fn test_relational_persistence() -> anyhow::Result<()> {
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

        let address_message_name = "address".to_string();
        let street = "street".to_string();
        let city = "city".to_string();
        let state = "state".to_string();
        let zip = "zip".to_string();
        mapper.add_mapping(
            address_message_name.clone(),
            street.clone(),
            address_message_name.clone(),
            street.clone(),
        )?;
        mapper.add_mapping(
            address_message_name.clone(),
            city.clone(),
            address_message_name.clone(),
            city.clone(),
        )?;
        mapper.add_mapping(
            address_message_name.clone(),
            state.clone(),
            address_message_name.clone(),
            state.clone(),
        )?;
        mapper.add_mapping(
            address_message_name.clone(),
            zip.clone(),
            address_message_name.clone(),
            zip.clone(),
        )?;
        mapper.add_relational_mapping("Contact", "address", "address", "id")?;
        let relational_message = serde_json::from_str(
            r#"
        {
            "first_name": "Gavin",
            "last_name": "Doughtie",
            "birth_year": "1990",
            "address": {
                "street": "123 Not Telling You",
                "city": "San Francisco",
                "state": "CA",
                "zip": "94000"
            }
        }
        "#,
        )
        .unwrap();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 1,
                },
                MockExecResult {
                    last_insert_id: 16,
                    rows_affected: 1,
                },
            ])
            .into_connection();
        let mut persister = DatabasePersister::new(db);
        let _record_one_id: u64 = mapper
            .persist_message(&mut persister, &message_name, &relational_message, None)
            .await?;
        let log = persister.db.into_transaction_log();
        println!("persisted:\n{:#?}", log);
        // let records_for_message = persister.tables.get(&message_name).unwrap();
        // let persisted_record_one = records_for_message.get(&record_one_id).unwrap();
        Ok(())
    }

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

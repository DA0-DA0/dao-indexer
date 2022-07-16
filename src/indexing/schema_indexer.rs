use std::collections::HashMap;

use crate::db::db_builder::DatabaseBuilder;

use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::{IndexerRegistry, RegistryKey};

use serde::{Deserialize, Serialize};

use log::{debug, warn};
use schemars::schema::{
    InstanceType, RootSchema, Schema, SchemaObject, SingleOrVec, SubschemaValidation,
};
// use tendermint::abci::transaction::Data;
use anyhow::anyhow;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct SchemaIndexerGenericMessage {}
impl IndexMessage for SchemaIndexerGenericMessage {
    fn index_message(&self, _registry: &IndexerRegistry, _events: &EventMap) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SchemaRef {
    pub name: String,
    pub schema: RootSchema,
    pub version: &'static str,
}

#[derive(Debug)]
pub struct SchemaIndexer {
    pub schemas: Vec<SchemaRef>,
    registry_keys: Vec<RegistryKey>,
    root_keys: Vec<String>,
    id: String,
}

type RootMap = HashMap<String, BTreeSet<String>>;

#[derive(Debug)]
pub struct SchemaData {
    pub root_keys: RootMap,
    pub required_roots: RootMap,
    pub optional_roots: RootMap,
    pub all_property_names: RootMap,
    pub sql_tables: HashMap<String, Vec<String>>,
    pub ref_roots: HashMap<String, String>,
    pub current_property: String,
}

impl SchemaData {
    pub fn default() -> Self {
        SchemaData {
            root_keys: HashMap::new(),
            required_roots: HashMap::new(),
            optional_roots: HashMap::new(),
            all_property_names: HashMap::new(),
            sql_tables: HashMap::new(),
            ref_roots: HashMap::new(),
            current_property: "".to_string(),
        }
    }
}

fn insert_table_set_value(
    table_values: &mut HashMap<String, BTreeSet<String>>,
    table_name: &str,
    value: &str,
) {
    if let Some(value_set) = table_values.get_mut(table_name) {
        value_set.insert(value.to_string());
        return;
    }
    let mut value_set = BTreeSet::new();
    value_set.insert(value.to_string());
    table_values.insert(table_name.to_string(), value_set);
}

impl SchemaIndexer {
    pub fn new(id: String, schemas: Vec<SchemaRef>) -> Self {
        SchemaIndexer {
            id: id.clone(),
            schemas,
            registry_keys: vec![RegistryKey::new(id)],
            root_keys: vec![],
        }
    }

    fn process_subschema(
        &self,
        subschema: &SubschemaValidation,
        name: &str,
        parent_name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        if let Some(all_of) = &subschema.all_of {
            for schema in all_of {
                match schema {
                    Schema::Object(schema_object) => {
                        db_builder
                            .column(parent_name, &format!("{}_id", name))
                            .integer();
                        self.process_schema_object(
                            schema_object,
                            parent_name,
                            name,
                            data,
                            db_builder,
                        )?;
                        // TODO(gavindoughtie): self.mapper.add_all_required(parent_name, name)
                    }
                    Schema::Bool(bool_val) => {
                        debug!("ignoring bool_val {} for {}", bool_val, name);
                    }
                }
            }
        } else if let Some(one_of) = &subschema.one_of {
            for schema in one_of {
                match schema {
                    Schema::Object(schema_object) => {
                        db_builder
                            .column(parent_name, &format!("{}_id", name))
                            .integer();
                        self.process_schema_object(
                            schema_object,
                            parent_name,
                            name,
                            data,
                            db_builder,
                        )?;
                        // TODO(gavindoughtie): self.mapper.add_one_required(parent_name, name)
                    }
                    Schema::Bool(bool_val) => {
                        debug!("ignoring bool_val {} for {}", bool_val, name);
                    }
                }
            }
        } else if let Some(any_of) = &subschema.any_of {
            for schema in any_of {
                match schema {
                    Schema::Object(schema_object) => {
                        db_builder
                            .column(parent_name, &format!("{}_id", name))
                            .integer();
                        self.process_schema_object(
                            schema_object,
                            parent_name,
                            name,
                            data,
                            db_builder,
                        )?;
                    }
                    Schema::Bool(bool_val) => {
                        debug!("ignoring bool_val {} for {}", bool_val, name);
                    }
                }
            }
        } else {
            println!("not handling subschema for {}", name);
        }
        Ok(())
    }

    fn update_root_map(&self, root_map: &mut RootMap, key: &str, value: &str) {
        if !root_map.contains_key(key) {
            root_map.insert(key.to_string(), BTreeSet::new());
        }
        if let Some(root) = root_map.get_mut(key) {
            root.insert(value.to_string());
        }
    }

    pub fn process_schema_object(
        &self,
        schema: &SchemaObject,
        parent_name: &str,
        name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        if let Some(reference) = &schema.reference {
            if !name.is_empty() {
                self.update_root_map(&mut data.required_roots, parent_name, name);
                data.ref_roots.insert(name.to_string(), reference.clone());
                return Ok(());
            }
        } else if let Some(subschema) = &schema.subschemas {
            self.process_subschema(subschema, name, parent_name, data, db_builder)?;
        }
        let table_name = name;
        if let Some(subschema) = &schema.subschemas {
            println!("{} is a subschema", name);
            return self.process_subschema(subschema, name, parent_name, data, db_builder);
        } else if schema.instance_type.is_none() {
            if schema.reference.is_none() {
                return Err(anyhow!("No instance or ref type for {}", name));
            }
            return Ok(());
        }
        let instance_type = schema.instance_type.as_ref().unwrap();
        match instance_type {
            SingleOrVec::Vec(vtype) => {
                println!("Vec instance for table {}, {:#?}", table_name, vtype);
            }
            SingleOrVec::Single(itype) => match itype.as_ref() {
                InstanceType::Object => {
                    let properties = &schema.object.as_ref().unwrap().properties;
                    let required = &schema.object.as_ref().unwrap().required;
                    for (property_name, schema) in properties {
                        if let Schema::Object(property_object_schema) = schema {
                            // println!("property_object_schema:\n{:#?}", property_object_schema);
                            if let Some(subschemas) = &property_object_schema.subschemas {
                                self.process_subschema(
                                    subschemas,
                                    property_name,
                                    name,
                                    data,
                                    db_builder,
                                )?;
                            }
                            if let Some(ref_property) = &property_object_schema.reference {
                                let backpointer_table_name = &ref_property[14..]; // Clip off "#/definitions/"
                                println!(
                                    r#"Adding relation from {}.{} back to {}"#,
                                    table_name, property_name, backpointer_table_name
                                );
                                db_builder.add_relation(
                                    table_name,
                                    property_name,
                                    backpointer_table_name,
                                )?;
                            }
                            if let Some(type_instance) = &property_object_schema.instance_type {
                                match type_instance {
                                    SingleOrVec::Single(single_val) => match **single_val {
                                        InstanceType::Object => {
                                            self.process_schema_object(
                                                property_object_schema,
                                                name,
                                                property_name,
                                                data,
                                                db_builder,
                                            )?;
                                            println!(
                                                "{}.{} is the ID of a {}",
                                                name, property_name, property_name
                                            );
                                        }
                                        InstanceType::Boolean => {
                                            // self.add_column_def(table_name, data, "BOOLEAN column".to_string());
                                            db_builder.column(table_name, property_name).boolean();
                                        }
                                        InstanceType::String => {
                                            db_builder.column(table_name, property_name).text();
                                        }
                                        InstanceType::Integer => {
                                            db_builder.column(table_name, property_name).integer();
                                        }
                                        InstanceType::Number => {
                                            // self.add_column_def(table_name, data, "Number column".to_string());
                                            db_builder.column(table_name, property_name).float();
                                        }
                                        InstanceType::Array => {
                                            println!(
                                                "not handling array instance for {}:{}",
                                                table_name, property_name
                                            );
                                        }
                                        InstanceType::Null => {
                                            println!(
                                                "not handling Null instance for {}:{}",
                                                table_name, property_name
                                            );
                                        }
                                    },
                                    SingleOrVec::Vec(vec_val) => {
                                        // Here we handle the case where we have a nullable field,
                                        // where vec_val[0] is the instance type and vec_val[1] is Null
                                        if vec_val.len() > 1
                                            && vec_val[vec_val.len() - 1] == InstanceType::Null
                                        {
                                            let optional_val = vec_val[0];
                                            match optional_val {
                                                InstanceType::Boolean => {
                                                    db_builder
                                                        .column(table_name, property_name)
                                                        .boolean();
                                                }
                                                InstanceType::String => {
                                                    // column_def = format!("{} TEXT", property_name);
                                                    db_builder
                                                        .column(table_name, property_name)
                                                        .text();
                                                }
                                                InstanceType::Integer => {
                                                    db_builder
                                                        .column(table_name, property_name)
                                                        .big_integer_len(78);
                                                }
                                                InstanceType::Number => {
                                                    db_builder
                                                        .column(table_name, property_name)
                                                        .big_integer_len(78);
                                                }
                                                InstanceType::Null => {
                                                    println!(
                                                        "Not handling Null type for {}",
                                                        property_name
                                                    );
                                                }
                                                InstanceType::Object => {
                                                    println!(
                                                        "Not handling Object type for {}",
                                                        property_name
                                                    );
                                                }
                                                InstanceType::Array => {
                                                    println!(
                                                        "Not handling Array type for {}",
                                                        property_name
                                                    );
                                                }
                                            }
                                        } else {
                                            warn!("unexpected");
                                        }
                                    }
                                }
                            }
                        }
                        data.current_property = property_name.clone();
                        self.update_root_map(
                            &mut data.all_property_names,
                            parent_name,
                            property_name,
                        );
                        insert_table_set_value(
                            &mut data.all_property_names,
                            table_name,
                            property_name,
                        );
                        if required.contains(property_name) {
                            insert_table_set_value(
                                &mut data.required_roots,
                                table_name,
                                property_name,
                            );
                        } else {
                            insert_table_set_value(
                                &mut data.optional_roots,
                                table_name,
                                property_name,
                            );
                        }
                    }
                }
                InstanceType::String => {
                    // self.add_column_def(table_name, data, format!("{} STRING column", name));
                    db_builder.column(table_name, name).string();
                }
                InstanceType::Null => {
                    warn!("Null instance type for {}/{}", table_name, name);
                    db_builder
                        .column(table_name, &format!("{}_id", name))
                        .integer();
                    // self.add_column_def(table_name, data, "Null column".to_string());
                }
                InstanceType::Boolean => {
                    // self.add_column_def(table_name, data, "BOOLEAN column".to_string());
                    db_builder.column(table_name, name).boolean();
                }
                InstanceType::Array => {
                    // self.add_column_def(table_name, data, "Array column".to_string());
                    warn!(
                        "Not handling Array instance type for {}/{}",
                        table_name, name
                    );
                }
                InstanceType::Number => {
                    // self.add_column_def(table_name, data, "Number column".to_string());
                    db_builder.column(table_name, name).float();
                }
                InstanceType::Integer => {
                    // self.add_column_def(table_name, data, "Integer column".to_string());
                    db_builder.column(table_name, name).integer();
                }
            },
        }
        Ok(())
    }
}

impl Indexer for SchemaIndexer {
    type MessageType = SchemaIndexerGenericMessage;
    fn id(&self) -> String {
        self.id.clone()
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter((self.root_keys).iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter([].into_iter())
    }
    // Indexes a message and its transaction events
    fn index<'a>(
        &'a self,
        // The registry of indexers
        _registry: &'a IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        _events: &'a EventMap,
        // Generic serde-parsed value dictionary
        _msg_dictionary: &'a Value,
        // The decoded string value of the message
        _msg_str: &'a str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn initialize_schemas<'a>(
        &'a mut self,
        builder: &'a mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        println!("initialize_schemas for schema_indexer {}", self.id);
        // let builder = &mut registry.db_builder;
        let schemas = self.schemas.clone();
        let mut visitor = SchemaVisitor::new(self, builder);
        for schema in schemas.iter() {
            visitor.visit_root_schema(&schema.schema)?;
        }
        Ok(())
    }
    // TODO(gavindoughtie): We can validate `msg` against the Schema and return our key
    // if it succeeds.
    fn extract_message_key(&self, _msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        Some(RegistryKey::new(self.id()))
    }
}

#[test]
fn test_schema_indexer_init() {
    use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
    use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
    use schemars::schema_for;

    let schema3 = schema_for!(Cw3DaoInstantiateMsg);
    let schema25 = schema_for!(Cw3DaoInstantiateMsg25);
    let indexer = SchemaIndexer::new(
        "Cw3DaoInstantiateMsg".to_string(),
        vec![
            SchemaRef {
                name: "Cw3DaoInstantiateMsg".to_string(),
                schema: schema3,
                version: "0.2.6",
            },
            SchemaRef {
                name: "Cw3DaoInstantiateMsg25".to_string(),
                schema: schema25,
                version: "0.2.5",
            },
        ],
    );
    println!("indexer:\n{:#?}", indexer);
}

pub struct SchemaVisitor<'a> {
    pub data: SchemaData,
    pub indexer: &'a mut SchemaIndexer,
    pub db_builder: &'a mut DatabaseBuilder,
}

impl<'a> SchemaVisitor<'a> {
    pub fn new(indexer: &'a mut SchemaIndexer, db_builder: &'a mut DatabaseBuilder) -> Self {
        SchemaVisitor {
            data: SchemaData::default(),
            indexer,
            db_builder,
        }
    }
    /// Override this method to modify a [`RootSchema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_root_schema`] function to visit subschemas.
    pub fn visit_root_schema(&mut self, root: &RootSchema) -> anyhow::Result<()> {
        let parent_name = self.indexer.id();
        for (root_def_name, schema) in root.definitions.iter() {
            if let Schema::Object(schema_object) = schema {
                self.indexer.process_schema_object(
                    schema_object,
                    root_def_name,
                    root_def_name,
                    &mut self.data,
                    self.db_builder,
                )?;
            } else {
                println!("Bool schema for {:#?}", schema);
            }
        }
        self.visit_schema_object(&root.schema, &parent_name)
    }

    /// Override this method to modify a [`Schema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_schema`] function to visit subschemas.
    pub fn visit_schema(&mut self, schema: &Schema, parent_name: &str) -> anyhow::Result<()> {
        if let Schema::Object(schema_val) = schema {
            return self.visit_schema_object(schema_val, parent_name);
        }
        Ok(())
    }

    pub fn visit_schema_object(
        &mut self,
        schema: &SchemaObject,
        parent_name: &str,
    ) -> anyhow::Result<()> {
        self.indexer.process_schema_object(
            schema,
            parent_name,
            parent_name,
            &mut self.data,
            self.db_builder,
        )
    }
}

use schemars::JsonSchema;
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
struct SimpleMessage {
    simple_field_one: String,
    simple_field_two: u128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
enum SimpleSubMessage {
    TypeA {
        type_a_contract_address: String,
        type_a_count: u32,
    },
    TypeB {
        type_b_contract_address: String,
        type_b_count: u32,
        type_b_addtional_field: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
struct SimpleRelatedMessage {
    title: String,
    message: SimpleMessage,
    sub_message: SimpleSubMessage,
}

#[allow(dead_code)]
fn get_test_registry(name: &str, schema: RootSchema) -> IndexerRegistry {
    use crate::indexing::indexer_registry::Register;
    let indexer = SchemaIndexer::new(
        name.to_string(),
        vec![SchemaRef {
            name: name.to_string(),
            schema,
            version: "0.0.0",
        }],
    );
    let mut registry = IndexerRegistry::default();
    registry.register(Box::from(indexer), None);
    registry
}

use sea_orm::sea_query::TableCreateStatement;
pub fn compare_table_create_statements(built_statement: &TableCreateStatement, expected_sql: &str) {
    use sea_orm::DbBackend;
    let db_postgres = DbBackend::Postgres;

    use sqlparser::ast::{ColumnDef, Statement};
    use sqlparser::dialect::PostgreSqlDialect;
    use sqlparser::parser::Parser;
    use std::collections::HashSet;

    let dialect = PostgreSqlDialect {}; // or AnsiDialect

    let built_sql = db_postgres.build(built_statement).to_string();
    println!("built:\n{}\nexpected:\n{}", built_sql, expected_sql);
    let built_ast = &Parser::parse_sql(&dialect, &built_sql).unwrap()[0];
    let expected_ast = &Parser::parse_sql(&dialect, expected_sql).unwrap()[0];

    // Because of the stupid non-deterministic nature of how the sql generation works, we
    // have to compare the members.

    if let Statement::CreateTable { columns, .. } = built_ast {
        let built_columns = HashSet::<ColumnDef>::from_iter(columns.iter().cloned());

        if let Statement::CreateTable { columns, .. } = expected_ast {
            let expected_columns = HashSet::<ColumnDef>::from_iter(columns.iter().cloned());
            assert_eq!(expected_columns, built_columns);
        }
    } else {
        panic!(r#"unable to compare sql"#);
    }
}

#[cfg(test)]
pub mod tests {
    use crate::db::db_persister::DatabasePersister;

    use super::*;
    use tokio::test;

    fn new_mock_persister() -> DatabasePersister {
        use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 1,
                },
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 1,
                },
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 1,
                },
            ])
            .into_connection();
        DatabasePersister::new(db)
    }

    #[test]
    async fn test_simple_message() {
        use schemars::schema_for;

        let name = stringify!(SimpleMessage);
        let schema = schema_for!(SimpleMessage);
        let mut registry = get_test_registry(name, schema);
        assert!(registry.initialize().is_ok(), "failed to init indexer");
        let built_table = registry.db_builder.table(name);
        let expected_sql = vec![
            r#"CREATE TABLE IF NOT EXISTS "simple_message" ("#,
            r#""simple_field_one" text,"#,
            r#""simple_field_two" integer"#,
            r#")"#,
        ]
        .join(" ");
        compare_table_create_statements(built_table, &expected_sql);

        let msg_str = r#"
        {
            "simple_field_one": "simple_field_one value",
            "simple_field_two": 33
        }"#;
        let msg_dictionary = serde_json::from_str(msg_str).unwrap();
        println!("msg_dictionary now:\n{:#?}", msg_dictionary);

        let mut persister = new_mock_persister();

        let result = registry
            .db_builder
            .value_mapper
            .persist_message(&mut persister, "SimpleMessage", &msg_dictionary, None)
            .await;

        println!("{:#?}", persister.db.into_transaction_log());
        assert!(result.is_ok());
        let result = registry.index_message_and_events(&EventMap::new(), &msg_dictionary, msg_str);
        assert!(result.is_ok());
    }

    #[test]
    async fn test_simple_related_message() {
        use schemars::schema_for;
        let name = stringify!(SimpleRelatedMessage);
        let schema = schema_for!(SimpleRelatedMessage);
        let mut registry = get_test_registry(name, schema);
        assert!(registry.initialize().is_ok(), "failed to init indexer");
        let expected_sql = vec![
            r#"CREATE TABLE IF NOT EXISTS "simple_related_message" ("#,
            r#""title" text,"#,
            r#""message_id" integer,"#,
            r#""sub_message_id" integer)"#,
        ]
        .join(" ");
        let built_table = registry.db_builder.table(name);
        compare_table_create_statements(built_table, &expected_sql);

        let msg_str = r#"
    {
        "title": "SimpleRelatedMessage Title",
        "message": {
            "simple_field_one": "simple_field_one value",
            "simple_field_two": 33
        },
        sub_message: {
            "type_a_contract_address": "type a contract address value",
            "type_a_count": 99
        },
    }"#;
        let msg_dictionary = serde_json::json!(msg_str);

        let mut persister = new_mock_persister();

        let result = registry
            .db_builder
            .value_mapper
            .persist_message(
                &mut persister,
                "SimpleRelatedMessage",
                &msg_dictionary,
                None,
            )
            .await;

        println!("{:#?}", persister.db.into_transaction_log());
        assert!(result.is_ok());
        let result = registry.index_message_and_events(&EventMap::new(), &msg_dictionary, msg_str);
        assert!(result.is_ok());
    }

    #[test]
    async fn test_visit() {
        use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
        use schemars::schema_for;
        let mut persister = new_mock_persister();
        let schema3 = schema_for!(Cw3DaoInstantiateMsg);
        let label = stringify!(Cw3DaoInstantiateMsg);
        let mut indexer = SchemaIndexer::new(label.to_string(), vec![]);
        let mut builder = DatabaseBuilder::new();
        let mut visitor = SchemaVisitor::new(&mut indexer, &mut builder);
        let result = visitor.visit_root_schema(&schema3);
        if result.is_err() {
            eprintln!("failed {:#?}", result);
        }
        let msg_string = r#"
    {
        "name": "Unit Test Dao",
        "description": "Unit Test Dao Description",
        "gov_token: {

        },
        "staking_contract": {

        },
        "threshold": {

        }
        "max_voting_period": {

        },
        "proposal_deposit_amount": {

        },
        "refund_failed_proposals": true,
        "image_url": "logo.png",
        "only_members_execute": true,
        "automatically_add_cw20s": true
    }
    "#;
        let msg = serde_json::json!(msg_string);
        let result = builder
            .value_mapper
            .persist_message(&mut persister, label, &msg, None)
            .await;
        assert!(result.is_ok());
    }
}

use std::collections::HashMap;

use crate::db::db_builder::DatabaseBuilder;
use crate::db::persister::PersisterRef;

use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::{IndexerRegistry, RegistryKey};

use serde::{Deserialize, Serialize};

use crate::db::db_util::foreign_key;
use anyhow::anyhow;
use log::{debug, warn};
use schemars::schema::{
    InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, SingleOrVec,
    SubschemaValidation,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct SchemaIndexerGenericMessage {}

#[allow(unused_variables)]
impl IndexMessage for SchemaIndexerGenericMessage {
    // This is a stub message; unlike the IndexMessage implemented for sepcific
    // messages, the SchemaIndexer itself performs indexing on its messages.
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
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
pub struct SchemaIndexer<T> {
    pub schemas: Vec<SchemaRef>,
    registry_keys: Vec<RegistryKey>,
    root_keys: Vec<String>,
    id: String,
    pub persister: PersisterRef<T>,
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

impl<T> SchemaIndexer<T> {
    pub fn new(id: String, schemas: Vec<SchemaRef>, persister: PersisterRef<T>) -> Self {
        SchemaIndexer {
            id: id.clone(),
            schemas,
            registry_keys: vec![RegistryKey::new(id)],
            root_keys: vec![],
            persister,
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
        println!("process_subschema {}->{}", parent_name, name);
        if let Some(all_of) = &subschema.all_of {
            for schema in all_of {
                match schema {
                    Schema::Object(schema_object) => {
                        db_builder.column(parent_name, &foreign_key(name)).integer();
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
                        // println!("process_subschema, subschema.one_of {}->{}\n{:#?}", parent_name, name, schema_object);
                        if let Some(obj) = &schema_object.object {
                            if let Some(name) = obj.required.iter().next() {
                                println!("Processing submessage {} on {}", name, parent_name);
                                let fk = foreign_key(name);
                                db_builder.column(parent_name, &fk).integer();
                                self.process_submessage(
                                    obj,
                                    schema_object,
                                    parent_name,
                                    name,
                                    data,
                                    db_builder,
                                )?;
                                // self.process_schema_object(
                                //     schema_object,
                                //     parent_name,
                                //     name,
                                //     data,
                                //     db_builder,
                                // )?;
                            }
                        }
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
                        // println!(
                        //     "process_subschema, subschema.any_of {}->{}",
                        //     parent_name, name
                        // );
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

    pub fn process_object_validation(
        &self,
        schema_obj_ref: &ObjectValidation,
        parent_name: &str,
        name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        let table_name = name; // TODO(gavin.doughtie): is this a spurious alias?
        let required = &schema_obj_ref.required;
        let properties = &schema_obj_ref.properties;
        for (property_name, schema) in properties {
            if let Schema::Object(property_object_schema) = schema {
                if let Some(subschemas) = &property_object_schema.subschemas {
                    self.process_subschema(
                        subschemas,
                        property_name,
                        table_name,
                        data,
                        db_builder,
                    )?;
                }
                if let Some(ref_property) = &property_object_schema.reference {
                    // Clip off "#/definitions/"
                    let backpointer_table_name = &ref_property["#/definitions/".len()..];
                    debug!(
                        r#"Adding relation from {}.{} back to {}"#,
                        table_name, property_name, backpointer_table_name
                    );
                    db_builder.add_relation(table_name, property_name, backpointer_table_name)?;
                }
                if let Some(type_instance) = &property_object_schema.instance_type {
                    match type_instance {
                        SingleOrVec::Single(single_val) => match **single_val {
                            InstanceType::Object => {
                                if table_name == property_name {
                                    // handle sub-messages by
                                    // setting the appropriate foreign key
                                    //source_table, source_property, destination_table
                                    db_builder.add_sub_message_relation(parent_name, property_name, table_name)?;                                
                                } else {
                                    db_builder.add_relation(table_name, property_name, name)?;
                                }
                                self.process_schema_object(
                                    property_object_schema,
                                    name,
                                    property_name,
                                    data,
                                    db_builder,
                                )?;
                            }
                            InstanceType::Boolean => {
                                db_builder.column(table_name, property_name).boolean();
                            }
                            InstanceType::String => {
                                db_builder.column(table_name, property_name).text();
                            }
                            InstanceType::Integer => {
                                db_builder.column(table_name, property_name).integer();
                            }
                            InstanceType::Number => {
                                db_builder.column(table_name, property_name).float();
                            }
                            InstanceType::Array => {
                                db_builder.many_many(table_name, property_name);
                                // eprintln!(
                                //     "not handling array instance for {}:{}",
                                //     table_name, property_name
                                // );
                            }
                            InstanceType::Null => {
                                eprintln!(
                                    "not handling Null instance for {}:{}",
                                    table_name, property_name
                                );
                            }
                        },
                        SingleOrVec::Vec(vec_val) => {
                            // Here we handle the case where we have a nullable field,
                            // where vec_val[0] is the instance type and vec_val[1] is Null
                            if vec_val.len() > 1 && vec_val[vec_val.len() - 1] == InstanceType::Null
                            {
                                let optional_val = vec_val[0];
                                match optional_val {
                                    InstanceType::Boolean => {
                                        db_builder.column(table_name, property_name).boolean();
                                    }
                                    InstanceType::String => {
                                        db_builder.column(table_name, property_name).text();
                                    }
                                    InstanceType::Integer => {
                                        db_builder.column(table_name, property_name).big_integer();
                                    }
                                    InstanceType::Number => {
                                        db_builder.column(table_name, property_name).big_integer();
                                    }
                                    InstanceType::Null => {
                                        eprintln!("Not handling Null type for {}", property_name);
                                    }
                                    InstanceType::Object => {
                                        eprintln!("Not handling Object type for {}", property_name);
                                    }
                                    InstanceType::Array => {
                                        eprintln!("Not handling Array type for {}", property_name);
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
            self.update_root_map(&mut data.all_property_names, parent_name, property_name);
            insert_table_set_value(&mut data.all_property_names, table_name, property_name);
            if required.contains(property_name) {
                insert_table_set_value(&mut data.required_roots, table_name, property_name);
            } else {
                insert_table_set_value(&mut data.optional_roots, table_name, property_name);
            }
        }
        Ok(())
    }

    pub fn process_submessage(
        &self,
        obj: &ObjectValidation,
        _schema: &SchemaObject,
        parent_name: &str,
        name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        println!("process_submessage {} on {}", name, parent_name);
        self.process_object_validation(obj, parent_name, name, data, db_builder)
        //  self.process_schema_object(schema, parent_name, name, data, db_builder)
    }

    pub fn process_schema_object(
        &self,
        schema: &SchemaObject,
        parent_name: &str,
        name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
        let table_name = name;
        if let Some(reference) = &schema.reference {
            if !name.is_empty() {
                self.update_root_map(&mut data.required_roots, parent_name, name);
                data.ref_roots.insert(name.to_string(), reference.clone());
                return Ok(());
            }
        } else if let Some(subschema) = &schema.subschemas {
            self.process_subschema(subschema, name, parent_name, data, db_builder)?;
            // } else if let Some(object_validation) = &schema.object {
            //     println!("What to do with {}?\n{:#?}", name, object_validation);
        }
        if schema.instance_type.is_none() {
            // This means we've popped to the top of the stack
            // of recursive calls to process the schema
            return Ok(());
        }

        let instance_type = schema
            .instance_type
            .as_ref()
            .ok_or_else(|| anyhow!("Unexpected empty instance_type"))?;

        match instance_type {
            SingleOrVec::Vec(vtype) => {
                println!("Vec instance for table {}, {:#?}", table_name, vtype);
            }
            SingleOrVec::Single(itype) => match itype.as_ref() {
                InstanceType::Object => {
                    let schema_obj_ref: &ObjectValidation = schema
                        .object
                        .as_ref()
                        .ok_or_else(|| anyhow!("no schema object"))?;
                    self.process_object_validation(
                        schema_obj_ref,
                        parent_name,
                        name,
                        data,
                        db_builder,
                    )?;
                }
                InstanceType::String => {
                    db_builder.column(table_name, name).string();
                }
                InstanceType::Null => {
                    warn!("Null instance type for {}/{}", table_name, name);
                    db_builder
                        .column(table_name, &format!("{}_id", name))
                        .integer();
                }
                InstanceType::Boolean => {
                    db_builder.column(table_name, name).boolean();
                }
                InstanceType::Array => {
                    warn!(
                        "Not handling Array instance type for {}/{}",
                        table_name, name
                    );
                }
                InstanceType::Number => {
                    db_builder.column(table_name, name).float();
                }
                InstanceType::Integer => {
                    db_builder.column(table_name, name).integer();
                }
            },
        }
        Ok(())
    }
}

impl Indexer for SchemaIndexer<u64> {
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
        registry: &'a IndexerRegistry,
        _events: &'a EventMap,
        msg_dictionary: &'a Value,
        _msg_str: &'a str,
    ) -> anyhow::Result<()> {
        if let Some(persister) = self.persister.try_write() {
            registry.db_builder.value_mapper.persist_message(
                persister.borrow().as_ref(),
                &self.id,
                msg_dictionary,
                None,
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("unable to get write lock"))
        }
    }

    fn initialize_schemas<'a>(
        &'a mut self,
        builder: &'a mut DatabaseBuilder,
    ) -> anyhow::Result<()> {
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

pub struct SchemaVisitor<'a> {
    pub data: SchemaData,
    pub indexer: &'a mut SchemaIndexer<u64>,
    pub db_builder: &'a mut DatabaseBuilder,
}

impl<'a> SchemaVisitor<'a> {
    pub fn new(indexer: &'a mut SchemaIndexer<u64>, db_builder: &'a mut DatabaseBuilder) -> Self {
        SchemaVisitor {
            data: SchemaData::default(),
            indexer,
            db_builder,
        }
    }
    /// Override this method to modify a [`RootSchema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`SchemaVisitor::visit_root_schema`] function to visit subschemas.
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
                eprintln!("Bool schema?");
            }
        }
        self.visit_schema_object(&root.schema, &parent_name)
    }

    /// Override this method to modify a [`Schema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`SchemaVisitor::visit_schema`] function to visit subschemas.
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

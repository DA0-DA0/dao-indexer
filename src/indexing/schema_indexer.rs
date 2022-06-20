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
    pub version: &'static str
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

fn insert_table_set_value(table_values: &mut HashMap<String, BTreeSet<String>>, table_name: &str, value: &str) {
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
        let mut indexer = SchemaIndexer {
            id: id.clone(),
            schemas,
            registry_keys: vec![RegistryKey::new(id)],
            root_keys: vec![],
        };
        indexer.init_from_schemas().unwrap();
        indexer
    }

    fn init_from_schemas(&mut self) -> anyhow::Result<()> {
        // let mut data_objects = vec![];
        // for schema in self.schemas.iter() {
        //     let mut data = SchemaData::default();
        //     self.process_schema_object(&schema.schema.schema, &schema.name, &mut data);
        //     data_objects.push(data);
        // }
        // debug!("schemas initialized:\n{:#?}", data_objects);
        // self.root_keys = data_objects[0].root_keys.clone();
        self.root_keys = vec![];
        Ok(())
    }

    pub fn get_column_def_unused(
        &self,
        property_name: &str,
        schema: &Schema,
        schema_name: &str,
        _parent_name: &str,
        data: &SchemaData,
    ) -> String {
        let mut column_def: String = "".to_string();
        let mut is_ref = false;
        let mut is_subschema = false;
        match schema {
            Schema::Object(schema) => {
                if let Some(reference) = &schema.reference {
                    is_ref = true;
                    column_def = format!("{} REFERENCE to {}", property_name, reference);
                }
                if let Some(_subschemas) = &schema.subschemas {
                    is_subschema = true;
                    let mut ref_key = "CANNOT FIND REF".to_string();
                    if let Some(ref_value) = data.ref_roots.get(property_name) {
                        ref_key = ref_value.to_string();
                    }
                    column_def = format!("{} SUBSCHEMA ({})", property_name, ref_key);
                }
                if let Some(type_instance) = &schema.instance_type {
                    match type_instance {
                        SingleOrVec::Single(single_val) => match *single_val.as_ref() {
                            InstanceType::Boolean => {
                                column_def = format!("{} BOOLEAN", property_name);
                            }
                            InstanceType::String => {
                                column_def = format!("{} TEXT NOT NULL", property_name);
                            }
                            InstanceType::Integer => {
                                column_def = format!("{} NUMERIC(78) NOT NULL", property_name);
                            }
                            InstanceType::Number => {
                                column_def = format!("{} NUMERIC(78) NOT NULL", property_name);
                            }
                            InstanceType::Object => {
                                column_def =
                                    format!("{}_{} REFERENCE OBJECT", schema_name, property_name);
                            }
                            InstanceType::Null => {
                                column_def = format!("{} NULL", property_name);
                            }
                            InstanceType::Array => {
                                column_def = format!("{} ARRAY", property_name);
                            }
                        },
                        SingleOrVec::Vec(vec_val) => {
                            // This is the test for an optional type:
                            if vec_val.len() > 1 && vec_val[vec_val.len() - 1] == InstanceType::Null
                            {
                                let optional_val = vec_val[0];
                                match optional_val {
                                    InstanceType::Boolean => {
                                        column_def = format!("{} BOOLEAN", property_name);
                                    }
                                    InstanceType::String => {
                                        column_def = format!("{} TEXT", property_name);
                                    }
                                    InstanceType::Integer => {
                                        column_def = format!("{} NUMERIC(78)", property_name);
                                    }
                                    InstanceType::Number => {
                                        column_def = format!("{} NUMERIC(78)", property_name);
                                    }
                                    _ => {
                                        column_def = format!(
                                            "{} {:?} Not handled",
                                            property_name, optional_val
                                        );
                                    }
                                }
                            } else {
                                warn!("unexpected");
                            }
                        }
                    }
                } else if !is_ref && !is_subschema {
                    println!("{} is neither a ref nor a known property", property_name);
                }
            }
            Schema::Bool(bool_val) => {
                column_def = format!("{} BOOLEAN {}", property_name, bool_val);
                println!("bool schema {} for {}", bool_val, property_name);
            }
        }
        column_def
    }

    fn process_subschema(
        &self,
        subschema: &SubschemaValidation,
        name: &str,
        parent_name: &str,
        data: &mut SchemaData,
        db_builder: &mut DatabaseBuilder,
    ) {
        if let Some(all_of) = &subschema.all_of {
            for schema in all_of {
                match schema {
                    Schema::Object(schema_object) => {
                        self.process_schema_object(schema_object, parent_name, name, data, db_builder);
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
                        self.process_schema_object(schema_object, parent_name, name, data, db_builder);
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
                        self.process_schema_object(schema_object, parent_name, name, data, db_builder);
                    }
                    Schema::Bool(bool_val) => {
                        debug!("ignoring bool_val {} for {}", bool_val, name);
                    }
                }
            }
        } else {
            println!("not handling subschema for {}", name);
        }
    }

    // fn add_column_def(&self, table_name: &str, data: &mut SchemaData, column_def: String) {
    //     let mut column_defs = data.sql_tables.get_mut(table_name);
    //     if column_defs.is_none() {
    //         data.sql_tables.insert(table_name.to_string(), vec![]);
    //         column_defs = data.sql_tables.get_mut(table_name);
    //     }
    //     if let Some(column_defs) = column_defs {
    //         column_defs.push(column_def);
    //     }
    // }

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
        db_builder: &mut DatabaseBuilder
    ) {
        if let Some(reference) = &schema.reference {
            if !name.is_empty() {
                self.update_root_map(&mut data.required_roots, parent_name, name);
                data.ref_roots.insert(name.to_string(), reference.clone());
            }
        } else if let Some(subschema) = &schema.subschemas {
            self.process_subschema(subschema, name, parent_name, data, db_builder);
        }
        if schema.instance_type.is_none() {
            eprintln!("No instance or ref type for {}", name);
            return;
        }
        let instance_type = schema.instance_type.as_ref().unwrap();
        let table_name = name;
        if let Some(subschema) = &schema.subschemas {
            println!("{} is a subschema", name);
            self.process_subschema(subschema, name, parent_name, data, db_builder);
        }
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
                            if let Some(subschemas) = &property_object_schema.subschemas {
                                self.process_subschema(subschemas, property_name, name, data, db_builder);
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
                                            println!("not handling array instance for {}:{}", table_name, property_name);
                                        }
                                        InstanceType::Null => {
                                            println!("not handling Null instance for {}:{}", table_name, property_name);
                                        }
                                        // _ => {
                                        //     eprintln!(
                                        //         "not handling single_val: {}/{}[{:#?}]",
                                        //         table_name, property_name, single_val
                                        //     );
                                        // }
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
                                                    // column_def = format!("{} TEXT", property_name);
                                                    db_builder.column(table_name, property_name).text();
                                                }
                                                InstanceType::Integer => {
                                                    db_builder.column(table_name, property_name).big_integer_len(78);
                                                }
                                                InstanceType::Number => {
                                                    db_builder.column(table_name, property_name).big_integer_len(78);
                                                }
                                                _ => {
                                                    println!(
                                                        "{} {:?} Not handled",
                                                        property_name, optional_val
                                                    );
                                                }
                                            }
                                        } else {
                                            warn!("unexpected");
                                        }
                                    }
                                    // _ => {
                                    //     debug!("Not worred about type {:?}", type_instance)
                                    // }
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
                        // TODO:recurse??
                        // db_builder.column(table_name, column_name).
                            // self.add_column_def(property_name, schema, name, parent_name, data, db_builder)?;
                        // if !column_def.is_empty() {
                        //     // let formatted_table_name = format!("{}_{}", parent_name, table_name);
                        //     self.add_column_def(table_name, data, column_def);
                        // } else if !is_subschema {
                        //     warn!(
                        //         "could not figure out a column def for property: {}, {:#?}",
                        //         property_name, schema
                        //     );
                        // }
                    }
                }
                InstanceType::String => {
                    // self.add_column_def(table_name, data, format!("{} STRING column", name));
                    db_builder.column(table_name, name).string();
                }
                InstanceType::Null => {
                    warn!("Null instance type for {}/{}", table_name, name);
                    // self.add_column_def(table_name, data, "Null column".to_string());
                }
                InstanceType::Boolean => {
                    // self.add_column_def(table_name, data, "BOOLEAN column".to_string());
                    db_builder.column(table_name, name).boolean();
                }
                InstanceType::Array => {
                    // self.add_column_def(table_name, data, "Array column".to_string());
                    warn!("Not handling Array instance type for {}/{}", table_name, name);
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
    fn initialize_schemas<'a>(&'a mut self, builder: &'a mut DatabaseBuilder) -> anyhow::Result<()>{
        println!("initialize_schemas for schema_indexer {}", self.id);
        // let builder = &mut registry.db_builder;
        let schemas = self.schemas.clone();
        let mut visitor = SchemaVisitor::new(self, builder);
        for schema in schemas.iter() {
            visitor.visit_root_schema(&schema.schema);
        }
        Ok(())
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
                version: "0.2.6"
            },
            SchemaRef {
                name: "Cw3DaoInstantiateMsg25".to_string(),
                schema: schema25,
                version: "0.2.5"
            },
        ],
    );
    println!("indexer:\n{:#?}", indexer);
}

pub struct SchemaVisitor<'a> {
    pub data: SchemaData,
    pub indexer: &'a mut SchemaIndexer,
    pub db_builder: &'a mut DatabaseBuilder
}

impl<'a> SchemaVisitor<'a> {
    pub fn new(indexer: &'a mut SchemaIndexer, db_builder: &'a mut DatabaseBuilder) -> Self {
        SchemaVisitor {
            data: SchemaData::default(),
            indexer,
            db_builder
        }
    }
    /// Override this method to modify a [`RootSchema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_root_schema`] function to visit subschemas.
    pub fn visit_root_schema(&mut self, root: &RootSchema) {
        let parent_name = self.indexer.id();
        for (root_def_name, schema) in root.definitions.iter() {
            if let Schema::Object(schema_object) = schema {
                self.indexer.process_schema_object(
                    schema_object,
                    &parent_name,
                    root_def_name,
                    &mut self.data,
                    self.db_builder
                )
            } else {
                println!("Bool schema for {:#?}", schema);
            }
        }
        self.visit_schema_object(&root.schema, &parent_name);
    }

    /// Override this method to modify a [`Schema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_schema`] function to visit subschemas.
    pub fn visit_schema(&mut self, schema: &Schema, parent_name: &str) {
        if let Schema::Object(schema_val) = schema {
            self.visit_schema_object(schema_val, parent_name);
        }
    }

    pub fn visit_schema_object(&mut self, schema: &SchemaObject, parent_name: &str) {
        self.indexer
            .process_schema_object(schema, parent_name, parent_name, &mut self.data, self.db_builder);
    }
}

#[test]
fn test_visit() {
    use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
    use schemars::schema_for;

    let schema3 = schema_for!(Cw3DaoInstantiateMsg);
    let label = stringify!(Cw3DaoInstantiateMsg);
    let mut indexer = SchemaIndexer::new(label.to_string(), vec![]);
    let mut builder = DatabaseBuilder::new();
    let mut visitor = SchemaVisitor::new(&mut indexer, &mut builder);
    visitor.visit_root_schema(&schema3);
    println!("indexer after visit: {:#?}", visitor.data);
}

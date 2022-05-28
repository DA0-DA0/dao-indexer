use std::collections::HashMap;

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
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct SchemaIndexerGenericMessage {}
impl IndexMessage for SchemaIndexerGenericMessage {
    fn index_message(&self, _registry: &IndexerRegistry, _events: &EventMap) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct SchemaRef {
    pub name: String,
    pub schema: RootSchema,
}

#[derive(Debug)]
pub struct SchemaIndexer {
    schemas: Vec<SchemaRef>,
    registry_keys: Vec<RegistryKey>,
    root_keys: Vec<String>,
    id: String,
}

#[derive(Debug)]
pub struct SchemaData {
    pub root_keys: Vec<String>,
    pub required_roots: BTreeSet<String>,
    pub optional_roots: BTreeSet<String>,
    pub all_property_names: BTreeSet<String>,
    pub column_defs: Vec<String>,
    pub table_creation_sql: Vec<String>,
    pub ref_roots: HashMap<String, String>,
}

impl SchemaData {
    pub fn default() -> Self {
        SchemaData {
            root_keys: vec![],
            required_roots: BTreeSet::new(),
            optional_roots: BTreeSet::new(),
            all_property_names: BTreeSet::new(),
            column_defs: vec![],
            table_creation_sql: vec![],
            ref_roots: HashMap::new(),
        }
    }
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
        let mut data_objects = vec![];
        for schema in self.schemas.iter() {
            let mut data = SchemaData::default();
            self.process_schema_object(&schema.schema.schema, &schema.name, &mut data);
            data_objects.push(data);
        }
        debug!("schemas initialized:\n{:#?}", data_objects);
        self.root_keys = data_objects[0].root_keys.clone();
        Ok(())
    }

    fn process_subschema(
        &self,
        subschema: &SubschemaValidation,
        name: &str,
        data: &mut SchemaData,
    ) {
        if let Some(all_of) = &subschema.all_of {
            for schema in all_of {
                match schema {
                    Schema::Object(schema_object) => {
                        self.process_schema_object(schema_object, name, data);
                    }
                    Schema::Bool(bool_val) => {
                        debug!("ignoring bool_val {} for {}", bool_val, name);
                    }
                }
            }
        }
    }

    pub fn process_schema_object(&self, schema: &SchemaObject, name: &str, data: &mut SchemaData) {
        if schema.is_ref() {
            debug!("Processing reference schema {} {}", name, &(schema.reference.unwrap()));
        }
        if schema.instance_type.is_none() {
            if let Some(reference) = &schema.reference {
                if !name.is_empty() {
                    data.required_roots.insert(name.to_string());
                    data.ref_roots.insert(name.to_string(), reference.clone());
                }
            } else {
                debug!("No instance or ref type for {}", name);
            }
            return;
        }
        let instance_type = schema.instance_type.as_ref().unwrap();
        let table_name = name;
        let mut is_subschema = false;
        match instance_type {
            SingleOrVec::Single(itype) => {
                match itype.as_ref() {
                    &InstanceType::Object => {
                        // println!("Yes, it's an object, properties:\n{:#?}", &(schema3.schema.object.unwrap().properties.keys().clone()));
                        let properties = &schema.object.as_ref().unwrap().properties;
                        let required = &schema.object.as_ref().unwrap().required;
                        for (property_name, schema) in properties {
                            // println!("property_name: {}", property_name);
                            data.all_property_names.insert(property_name.clone());
                            if required.contains(property_name) {
                                data.required_roots.insert(property_name.clone());
                            } else {
                                data.optional_roots.insert(property_name.clone());
                            }
                            let mut column_def: String = "".to_string();
                            match schema {
                                schemars::schema::Schema::Object(schema) => {
                                    match &schema.instance_type {
                                        Some(type_instance) => {
                                            match type_instance {
                                                SingleOrVec::Single(single_val) => {
                                                    // println!("Single value");
                                                    // data.required_roots.push(property_name.clone());
                                                    match *single_val.as_ref() {
                                                        InstanceType::Boolean => {
                                                            column_def = format!(
                                                                "{} BOOLEAN",
                                                                property_name
                                                            );
                                                        }
                                                        InstanceType::String => {
                                                            column_def = format!(
                                                                "{} TEXT NOT NULL",
                                                                property_name
                                                            );
                                                        }
                                                        InstanceType::Integer => {
                                                            column_def = format!(
                                                                "{} NUMERIC(78) NOT NULL",
                                                                property_name
                                                            );
                                                        }
                                                        InstanceType::Number => {
                                                            column_def = format!(
                                                                "{} NUMERIC(78) NOT NULL",
                                                                property_name
                                                            );
                                                        }
                                                        _ => {
                                                            warn!("{:?} Not handled", single_val);
                                                        }
                                                    }
                                                }
                                                SingleOrVec::Vec(vec_val) => {
                                                    // println!("Vec value {:#?}", vec_val);
                                                    // This is the test for an optional type:
                                                    if vec_val.len() > 1
                                                        && vec_val[vec_val.len() - 1]
                                                            == InstanceType::Null
                                                    {
                                                        let optional_val = vec_val[0];
                                                        match optional_val {
                                                            InstanceType::Boolean => {
                                                                column_def = format!(
                                                                    "{} BOOLEAN",
                                                                    property_name
                                                                );
                                                            }
                                                            InstanceType::String => {
                                                                column_def = format!(
                                                                    "{} TEXT",
                                                                    property_name
                                                                );
                                                            }
                                                            InstanceType::Integer => {
                                                                column_def = format!(
                                                                    "{} NUMERIC(78)",
                                                                    property_name
                                                                );
                                                            }
                                                            InstanceType::Number => {
                                                                column_def = format!(
                                                                    "{} NUMERIC(78)",
                                                                    property_name
                                                                );
                                                            }
                                                            _ => {
                                                                warn!(
                                                                    "{:?} Not handled",
                                                                    optional_val
                                                                );
                                                            }
                                                        }
                                                    } else {
                                                        warn!("unexpected");
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            // println!("{} has no instance_type", property_name);
                                            // data.required_roots.push(property_name.clone());
                                            if let Some(subschema) = &schema.subschemas {
                                                is_subschema = true;
                                                self.process_subschema(
                                                    subschema,
                                                    property_name,
                                                    data,
                                                );
                                            } else {
                                                warn!(
                                                    "process schema {}, {:#?}",
                                                    property_name, schema
                                                );
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    warn!("Not an object type: {:#?}", schema);
                                }
                            }
                            if !column_def.is_empty() {
                                data.column_defs.push(column_def);
                            } else if !is_subschema {
                                warn!(
                                    "could not figure out a column def for property: {}, {:#?}",
                                    property_name, schema
                                );
                            }
                        }
                        // println!("property details:\n{:#?}", properties);
                        let create_table_sql = format!(
                            "CREATE_TABLE {} (\n{}\n);\n",
                            table_name,
                            data.column_defs.join(",\n")
                        );
                        // println!("SQL:\n{}", create_table_sql);
                        data.table_creation_sql.push(create_table_sql);
                    }
                    _ => {
                        debug!("god only knows");
                    }
                }
            }
            _ => {
                debug!("not object");
            }
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
            },
            SchemaRef {
                name: "Cw3DaoInstantiateMsg25".to_string(),
                schema: schema25,
            },
        ],
    );
    println!("indexer:\n{:#?}", indexer);
}

pub struct SchemaVisitor {
    pub data: SchemaData,
    pub indexer: SchemaIndexer,
}

impl SchemaVisitor {
    pub fn new(indexer: SchemaIndexer) -> Self {
        SchemaVisitor {
            data: SchemaData::default(),
            indexer,
        }
    }
}

use schemars::visit::{visit_root_schema, visit_schema, visit_schema_object, Visitor};

impl Visitor for SchemaVisitor {
    /// Override this method to modify a [`RootSchema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_root_schema`] function to visit subschemas.
    fn visit_root_schema(&mut self, root: &mut RootSchema) {
        for (root_def_name, schema) in root.definitions.iter() {
            println!("visting root definition {}", root_def_name);
            if let Schema::Object(schema_object) = schema {
              self.indexer.process_schema_object(schema_object, root_def_name, &mut self.data)
            }
        }
        visit_root_schema(self, root)
    }

    /// Override this method to modify a [`Schema`] and (optionally) its subschemas.
    ///
    /// When overriding this method, you will usually want to call the [`visit_schema`] function to visit subschemas.
    fn visit_schema(&mut self, schema: &mut Schema) {
        if let Schema::Object(schema_val) = schema {
            self.indexer
                .process_schema_object(schema_val, "", &mut self.data);
        }
        visit_schema(self, schema)
    }

    fn visit_schema_object(&mut self, schema: &mut SchemaObject) {
        // // First, make our change to this schema
        // schema
        //     .extensions
        //     .insert("my_property".to_string(), serde_json::json!("hello world"));

        self.indexer
            .process_schema_object(schema, "", &mut self.data);
        // Then delegate to default implementation to visit any subschemas
        visit_schema_object(self, schema);
    }
}

#[test]
fn test_visit() {
    use cw3_dao::msg::GovTokenMsg;
    use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
    use schemars::schema_for;

    let mut schema3 = schema_for!(Cw3DaoInstantiateMsg);
    let label = stringify!(Cw3DaoInstantiateMsg);
    let indexer = SchemaIndexer::new(
        label.to_string(),
        vec![SchemaRef {
            name: label.to_string(),
            schema: schema_for!(GovTokenMsg),
        }],
    );
    let mut visitor = SchemaVisitor::new(indexer);
    visitor.visit_root_schema(&mut schema3);
    println!("indexer after visit: {:#?}", visitor.data);
    // let schema3 = schema_for!(Cw3DaoInstantiateMsg);
    // let string_schema = serde_json::to_string_pretty(&schema3).unwrap();
    // println!("string_schema:\n{}", string_schema);
}

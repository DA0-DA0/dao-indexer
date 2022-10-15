use log::{debug, warn};
use schemars::schema::{
    InstanceType, RootSchema, Schema, SchemaObject, SingleOrVec, SubschemaValidation,
};

pub fn dump_subschema(subschema: &SubschemaValidation, name: &str) {
    if let Some(all_of) = &subschema.all_of {
        for schema in all_of {
            match schema {
                Schema::Object(schema_object) => {
                    dump_schema_object(schema_object, name);
                }
                Schema::Bool(bool_val) => {
                    debug!("ignoring bool_val {} for {}", bool_val, name);
                }
            }
        }
    }
}

pub fn dump_schema_object(schema: &SchemaObject, name: &str) {
    if schema.instance_type.is_none() {
        if let Some(reference) = &schema.reference {
            println!("No instance type, but ref: {}", reference);
        } else {
            println!("No instance or ref type for {}", name);
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
                    let properties = &schema.object.as_ref().unwrap().properties;
                    let mut required_roots = vec![];
                    let mut optional_roots = vec![];
                    let mut all_property_names = vec![];
                    let mut column_defs = vec![];
                    for (property_name, schema) in properties {
                        all_property_names.push(property_name);
                        let mut column_def: String = "".to_string();
                        match schema {
                            schemars::schema::Schema::Object(schema) => {
                                match &schema.instance_type {
                                    Some(type_instance) => {
                                        match type_instance {
                                            SingleOrVec::Single(single_val) => {
                                                // println!("Single value");
                                                required_roots.push(property_name);
                                                match *single_val.as_ref() {
                                                    InstanceType::Boolean => {
                                                        column_def =
                                                            format!("{} BOOLEAN", property_name);
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
                                                        println!("{:?} Not handled", single_val);
                                                    }
                                                }
                                            }
                                            SingleOrVec::Vec(vec_val) => {
                                                // This is the test for an optional type:
                                                if vec_val.len() > 1
                                                    && vec_val[vec_val.len() - 1]
                                                        == InstanceType::Null
                                                {
                                                    optional_roots.push(property_name);
                                                    let optional_val = vec_val[0];
                                                    match optional_val {
                                                        InstanceType::Boolean => {
                                                            column_def = format!(
                                                                "{} BOOLEAN",
                                                                property_name
                                                            );
                                                        }
                                                        InstanceType::String => {
                                                            column_def =
                                                                format!("{} TEXT", property_name);
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
                                                            eprintln!(
                                                                "{:?} Not handled",
                                                                optional_val
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    println!("unexpected");
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        required_roots.push(property_name);
                                        if let Some(subschema) = &schema.subschemas {
                                            is_subschema = true;
                                            dump_subschema(subschema, property_name);
                                        } else {
                                            debug!(
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
                            column_defs.push(column_def);
                        } else if !is_subschema {
                            println!(
                                "could not figure out a column def for property: {}, {:#?}",
                                property_name, schema
                            );
                        }
                    }
                    println!(
                        "required roots:\n{:#?}\noptional roots:\n{:#?}\nall:\n{:#?}",
                        required_roots, optional_roots, all_property_names
                    );
                    let create_table_sql = format!(
                        "CREATE_TABLE {} (\n{}\n);\n",
                        table_name,
                        column_defs.join(",\n")
                    );
                    println!("SQL:\n{}", create_table_sql);
                }
                _ => {
                    println!("god only knows");
                }
            }
        }
        _ => {
            println!("not object");
        }
    }
}

pub fn dump_schema(root_schema: &RootSchema, name: &str) {
    dump_schema_object(&root_schema.schema, name);
}

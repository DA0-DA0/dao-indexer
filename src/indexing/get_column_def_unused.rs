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

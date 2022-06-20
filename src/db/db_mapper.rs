use serde_json::Value;

pub struct DatabaseMapper {}

impl DatabaseMapper {
    pub fn add_mapping(message_name: &str, field_name: &str, table_name: &str, column_name: &str) {
        println!(
            "add_mapping(message_name: {}, field_name: {}, table_name: {}, column_name: {})",
            message_name, field_name, table_name, column_name
        );
    }

    pub fn add_relational_mapping(
        message_name: &str,
        field_name: &str,
        table_name: &str,
        column_name: &str,
    ) {
        println!("add_mapping(add_relational_mapping: {}, field_name: {}, table_name: {}, column_name: {})", message_name, field_name, table_name, column_name);
    }

    fn keys(&self) -> Vec<String> {
        vec![]
    }

    pub fn persist_message(&mut self, table_name: &str, msg: &Value) {
        println!("persist_msg {}, {:#?}", table_name, msg);

        for key in self.keys() {
            if let Some(Value::String(val)) = msg.get(&key) {
                println!("Saving {}:{}={}", table_name, key, val);
            }
        }
    }
}

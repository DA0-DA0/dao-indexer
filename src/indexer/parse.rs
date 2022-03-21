extern crate serde_json;

use serde_json::Value;

pub fn parse_message(msg: &[u8]) -> Result<Option<Value>, Box<dyn std::error::Error>> {
  match String::from_utf8(msg.to_owned()) {
    Ok(exec_msg_str) => match serde_json::from_str::<Value>(&exec_msg_str) {
      Ok(parsed) => {
        Ok(Some(parsed))
      }
      Err(e) => {
        Err(Box::from(format!("Error parsing {}", e)))
      }
    },
    Err(e) => {
      Err(Box::from(format!("Error unpacking utf8 {:?}", e)))
    }
  }
}

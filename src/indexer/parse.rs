use serde_json::Value;

pub fn parse_message(msg: &[u8]) -> serde_json::Result<Option<Value>> {
  match String::from_utf8(msg.to_owned()) {
    Ok(exec_msg_str) => serde_json::from_str(&exec_msg_str)
  }  
}

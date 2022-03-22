use cw3_dao::msg::ExecuteMsg;
use std::collections::BTreeMap;
pub fn dump_execute_contract(execute_contract: &ExecuteMsg) {
  println!("handle execute contract {:?}", execute_contract);
}

pub fn dump_events(events: &Option<BTreeMap<String, Vec<String>>>) {
  if let Some(event_map) = events {
    println!("************* vv Events ***********");
    for (key, value) in event_map {
      println!("{} / {:?}", key, value);
    }
    println!("************* ^^ Events ***********");
  }
}

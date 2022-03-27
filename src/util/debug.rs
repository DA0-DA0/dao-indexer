use crate::indexing::event_map::EventMap;
use cw3_dao::msg::ExecuteMsg;

pub fn dump_execute_contract(execute_contract: &ExecuteMsg) {
    println!("handle execute contract {:?}", execute_contract);
}

pub fn dump_events(events: &EventMap) {
    println!("************* vv Events ***********");
    for (key, value) in events {
        println!("{} / {:?}", key, value);
    }
    println!("************* ^^ Events ***********");
}

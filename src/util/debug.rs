use crate::indexing::event_map::EventMap;
use cw3_dao::msg::ExecuteMsg;
use log::debug;

pub fn dump_execute_contract(execute_contract: &ExecuteMsg) {
    debug!("handle execute contract {:?}", execute_contract);
}

pub fn events_string(events: &EventMap) -> String {
    let mut output = String::default();
    output.push_str("\n************* vv Events ***********\n");
    for (key, value) in events {
        output.push_str(&format!("  {}: {:?}\n", key, value));
    }
    output.push_str("************* ^^ Events ***********\n");
    output
}

pub fn dump_events(events: &EventMap) {
    debug!("{}", &events_string(events));
}

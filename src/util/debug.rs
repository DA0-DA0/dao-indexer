use crate::indexing::event_map::EventMap;
use cw3_dao::msg::ExecuteMsg;
use log::debug;
use std::fmt::Write as _; // import without risk of name clashing

pub fn dump_execute_contract(execute_contract: &ExecuteMsg) {
    debug!("handle execute contract {:?}", execute_contract);
}

pub fn events_string(events: &EventMap) -> String {
    let mut output = String::default();
    output.push_str("\n************* vv Events ***********\n");
    for (key, value) in events {
        let _ = writeln!(output, "  {}: {:?}", key, value);
    }
    output.push_str("************* ^^ Events ***********\n");
    output
}

pub fn dump_events(events: &EventMap) {
    debug!("{}", &events_string(events));
}

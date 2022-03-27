use crate::indexing::event_map::EventMap;
use cw3_dao::msg::ExecuteMsg;
use log::debug;

pub fn dump_execute_contract(execute_contract: &ExecuteMsg) {
    debug!("handle execute contract {:?}", execute_contract);
}

pub fn dump_events(events: &EventMap) {
    debug!("************* vv Events ***********");
    for (key, value) in events {
        debug!("{} / {:?}", key, value);
    }
    debug!("************* ^^ Events ***********");
}

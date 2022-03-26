use super::index::Index;
use super::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
use stake_cw20::msg::ExecuteMsg;
use std::collections::BTreeMap;

impl Index for ExecuteMsg {
    fn index(
        &self,
        _registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("StakeCw20ExecuteMsg index");
        dump_events(events);
        Ok(())
    }
}

use crate::indexing::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
use log::debug;
use stake_cw20::msg::ExecuteMsg;

impl IndexMessage for ExecuteMsg {
    fn index_message(&self, _registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        debug!("StakeCw20ExecuteMsg index");
        dump_events(events);
        Ok(())
    }
}

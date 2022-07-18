use crate::indexing::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
pub use cw3_multisig::msg::ExecuteMsg;
use log::warn;

impl IndexMessage for ExecuteMsg {
    fn index_message(
        &self,
        _registry: &IndexerRegistry,
        event_map: &EventMap,
    ) -> anyhow::Result<()> {
        dump_events(event_map);
        warn!("index_message not implemented for cw3_multisig ExecuteMsg");
        Ok(())
    }
}

use crate::indexing::debug::index_message;
use crate::indexing::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use crate::indexing::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
pub use cw20::Cw20ExecuteMsg;

impl IndexMessage for MsgSend {
    fn index_message(&self, _registry: &IndexerRegistry, _events: &EventMap) -> anyhow::Result<()> {
        index_message(&self.from_address, &self.to_address, &self.amount, None)
    }
}

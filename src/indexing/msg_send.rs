use super::debug::index_message;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
pub use cw20::Cw20ExecuteMsg;
use std::collections::BTreeMap;

impl IndexMessage for MsgSend {
    fn index_message(
        &self,
        _registry: &IndexerRegistry,
        _events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        index_message(&self.from_address, &self.to_address, &self.amount, None)
    }
}

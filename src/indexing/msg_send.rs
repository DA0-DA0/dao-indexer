use super::debug::index_message;
use super::index::Index;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
pub use cw20::Cw20ExecuteMsg;
use std::collections::BTreeMap;

impl Index for MsgSend {
    fn index(
        &self,
        registry: &IndexerRegistry,
        _events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        index_message(
            &registry.db,
            &self.from_address,
            &self.to_address,
            &self.amount,
            None,
        )
    }
}

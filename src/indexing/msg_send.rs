use super::debug::index_message;
use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
pub use cw20::Cw20ExecuteMsg;
use diesel::PgConnection;

impl IndexMessage for MsgSend {
    fn index_message(&self, _conn: Option<&PgConnection>, _registry: &IndexerRegistry, _events: &EventMap) -> anyhow::Result<()> {
        index_message(&self.from_address, &self.to_address, &self.amount, None)
    }
}

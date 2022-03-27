use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
pub use cw20::Cw20ExecuteMsg;
use serde_json::Value;

impl IndexMessage for MsgExecuteContract {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        events: &EventMap,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg_str = String::from_utf8(self.msg.clone())?;
        let msg_val: Value = serde_json::from_str(&msg_str)?;
        registry.index_message_and_events(events, &msg_val, &msg_str)
    }
}

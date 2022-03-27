use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer::Indexer;
use super::indexer_registry::IndexerRegistry;
use super::indexer_registry::RegistryKey;
pub use cw20::Cw20ExecuteMsg;
use serde_json::Value;

const INDEXER_KEY: &str = "Cw20ExecuteMsg";
static ROOT_KEYS: [&str; 11] = [
    "send",
    "burn",
    "transfer",
    "increase_allowance",
    "decrease_allowance",
    "transfer_from",
    "send_from",
    "burn_from",
    "mint",
    "update_marketing",
    "upload_logo",
];

pub struct Cw20ExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw20ExecuteMsgIndexer {
    fn default() -> Self {
        Cw20ExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(INDEXER_KEY)],
        }
    }
}

impl Indexer for Cw20ExecuteMsgIndexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &EventMap,
        _msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let execute_contract = serde_json::from_str::<Cw20ExecuteMsg>(msg_str)?;
        execute_contract.index_message(registry, events)
    }
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> std::slice::Iter<RegistryKey> {
        self.registry_keys.iter()
    }
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        for key in ROOT_KEYS {
            if msg.get(key).is_some() {
                return Some(RegistryKey::new(&self.id()));
            }
        }
        None
    }
}

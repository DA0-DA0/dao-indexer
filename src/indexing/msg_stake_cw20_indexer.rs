use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer::Indexer;
use super::indexer_registry::{IndexerRegistry, RegistryKey};
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;

const INDEXER_KEY: &str = "StakeCw20ExecuteMsg";
static ROOT_KEYS: [&str; 4] = ["receive", "unstake", "claim", "update_config"];

pub struct StakeCw20ExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for StakeCw20ExecuteMsgIndexer {
    fn default() -> Self {
        StakeCw20ExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(INDEXER_KEY)],
        }
    }
}

impl Indexer for StakeCw20ExecuteMsgIndexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &EventMap,
        _msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let execute_contract = serde_json::from_str::<StakeCw20ExecuteMsg>(msg_str)?;
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

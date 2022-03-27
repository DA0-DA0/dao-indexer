use super::indexer::Indexer;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;

const INDEXER_KEY: &str = "StakeCw20ExecuteMsg";
static ROOT_KEYS: [&str; 4] = [
    "receive", "unstake", "claim", "update_config"
];

pub struct StakeCw20ExecuteMsgIndexer {
    registry_keys: Vec<String>,
}

impl Default for StakeCw20ExecuteMsgIndexer {
    fn default() -> Self {
        StakeCw20ExecuteMsgIndexer {
            registry_keys: vec![INDEXER_KEY.to_string()],
        }
    }
}

impl Indexer for StakeCw20ExecuteMsgIndexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
        _msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let execute_contract = serde_json::from_str::<StakeCw20ExecuteMsg>(msg_str)?;
        execute_contract.index_message(registry, events)
    }
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> std::slice::Iter<String> {
        self.registry_keys.iter()
    }
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<String> {
        for key in ROOT_KEYS {
            if msg.get(key).is_some() {
                return Some(self.id());
            }
        }
        None
    }
}

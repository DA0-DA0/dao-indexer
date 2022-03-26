use super::index::Index;
use super::indexer::Indexer;
use super::indexer_registry::IndexerRegistry;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;

pub struct StakeCw20ExecuteMsgIndexer {
    my_registry_keys: Vec<String>,
}

impl Default for StakeCw20ExecuteMsgIndexer {
    fn default() -> Self {
        StakeCw20ExecuteMsgIndexer {
            my_registry_keys: vec!["StakeCw20ExecuteMsg".to_string()],
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
        execute_contract.index(registry, events)
    }
    fn id(&self) -> String {
        "StakeCw20ExecuteMsgIndexer".to_string()
    }
    fn registry_keys(&self) -> std::slice::Iter<String> {
        self.my_registry_keys.iter()
    }
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<String> {
        let root_keys = vec!["receive", "unstake", "claim", "update_config"];
        for key in root_keys {
            if msg.get(key).is_some() {
                return Some("StakeCw20ExecuteMsg".to_string());
            }
        }
        None
    }
}

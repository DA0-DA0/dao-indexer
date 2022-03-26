use super::index::Index;
use super::indexer::Indexer;
use super::indexer_registry::IndexerRegistry;
pub use cw20::Cw20ExecuteMsg;
use serde_json::Value;
use std::collections::BTreeMap;

pub struct Cw20ExecuteMsgIndexer {
    my_registry_keys: Vec<String>,
}

impl Default for Cw20ExecuteMsgIndexer {
    fn default() -> Self {
        Cw20ExecuteMsgIndexer {
            my_registry_keys: vec!["Cw20ExecuteMsg".to_string()],
        }
    }
}

impl Indexer for Cw20ExecuteMsgIndexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
        _msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let execute_contract = serde_json::from_str::<Cw20ExecuteMsg>(msg_str)?;
        execute_contract.index(registry, events)
    }
    fn id(&self) -> String {
        "Cw20ExecuteMsgIndexer".to_string()
    }
    fn registry_keys(&self) -> std::slice::Iter<String> {
        self.my_registry_keys.iter()
    }
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<String> {
        let cw20_root_keys = vec![
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
            // `propose`, `vote`, `execute`, `close`, `pause_d_a_o`, `update_config`, `update_cw20_token_list`, `update_staking_contract`, `receive`
        ];
        for key in cw20_root_keys {
            if msg.get(key).is_some() {
                return Some("Cw20ExecuteMsg".to_string());
            }
        }
        None
    }
}

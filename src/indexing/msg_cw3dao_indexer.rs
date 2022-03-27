use super::indexer::Indexer;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use serde_json::Value;
use std::collections::BTreeMap;

const INDEXER_KEY: &str = "Cw3DaoExecuteMsg";
static ROOT_KEYS: [&str; 9] = [
    "propose",
    "vote",
    "execute",
    "close",
    "pause_d_a_o",
    "update_config",
    "update_cw20_token_list",
    "update_staking_contract",
    "receive",
];

pub struct Cw3DaoExecuteMsgIndexer {
    registry_keys: Vec<String>,
}

impl Default for Cw3DaoExecuteMsgIndexer {
    fn default() -> Self {
        Cw3DaoExecuteMsgIndexer {
            registry_keys: vec![INDEXER_KEY.to_string()],
        }
    }
}

impl Indexer for Cw3DaoExecuteMsgIndexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
        _msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let execute_contract = serde_json::from_str::<Cw3DaoExecuteMsg>(msg_str)?;
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

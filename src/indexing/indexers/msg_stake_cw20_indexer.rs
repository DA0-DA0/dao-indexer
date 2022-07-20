use crate::indexing::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use crate::indexing::indexer_registry::RegistryKey;
use log::debug;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;

const INDEXER_KEY: &str = "StakeCw20ExecuteMsg";

pub struct StakeCw20ExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
    root_keys: Vec<String>,
}

impl Default for StakeCw20ExecuteMsgIndexer {
    fn default() -> Self {
        StakeCw20ExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(INDEXER_KEY.to_string())],
            root_keys: vec![
                "receive".to_string(),
                "unstake".to_string(),
                "claim".to_string(),
                "update_config".to_string(),
            ],
        }
    }
}

impl Indexer for StakeCw20ExecuteMsgIndexer {
    type MessageType = StakeCw20ExecuteMsg;
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter(self.root_keys.iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter([].into_iter())
    }

    // Extract the key from a given message. This should be one of the keys
    // returned in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        if msg.get("unstake").is_some() && msg.get("amount").is_none() {
            debug!(
                "msg_stake_cw20_indexer ignoring non-amount unstake message\n{:#?}",
                msg
            );
            return None;
        }
        self.first_matching_key(msg)
    }
}

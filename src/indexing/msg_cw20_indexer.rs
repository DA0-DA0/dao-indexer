use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};

use super::indexer_registry::RegistryKey;
pub use cw20::Cw20ExecuteMsg;
use log::debug;
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
            registry_keys: vec![RegistryKey::new(INDEXER_KEY.to_string())],
        }
    }
}

impl Indexer for Cw20ExecuteMsgIndexer {
    type MessageType = Cw20ExecuteMsg;
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter(ROOT_KEYS.into_iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter([].into_iter())
    }

    // Extract the key from a given message. This should be one of the keys
    // returned in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        if msg.get("mint").is_some() && msg.get("recipient").is_none() {
            debug!(
                "msg_cw20_indexer ignoring non-token mint message\n{:#?}",
                msg
            );
            return None;
        }
        let roots = self.root_keys();
        for key in roots {
            if msg.get(key).is_some() {
                return Some(RegistryKey::new(self.id()));
            }
        }
        None
    }
}

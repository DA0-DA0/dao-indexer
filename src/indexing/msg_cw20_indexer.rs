use super::indexer::Indexer;

use super::indexer_registry::RegistryKey;
pub use cw20::Cw20ExecuteMsg;
use std::slice::Iter;

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
    type MessageType = Cw20ExecuteMsg;
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> Iter<RegistryKey> {
        self.registry_keys.iter()
    }
    fn root_keys(&self) -> Iter<&str> {
        ROOT_KEYS.iter()
    }
}

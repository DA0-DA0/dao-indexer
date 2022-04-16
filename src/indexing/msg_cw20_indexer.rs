use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};

use super::indexer_registry::RegistryKey;
pub use cw20::Cw20ExecuteMsg;

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
}

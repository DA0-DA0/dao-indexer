use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::RegistryKey;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;

const INDEXER_KEY: &str = "StakeCw20ExecuteMsg";
static ROOT_KEYS: [&str; 4] = ["receive", "unstake", "claim", "update_config"];

pub struct StakeCw20ExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for StakeCw20ExecuteMsgIndexer {
    fn default() -> Self {
        StakeCw20ExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(INDEXER_KEY.to_string())],
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
        root_keys_from_iter(ROOT_KEYS.into_iter())
    }
}

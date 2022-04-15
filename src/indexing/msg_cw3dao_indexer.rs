use super::indexer::Indexer;
use super::indexer_registry::RegistryKey;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use std::slice::Iter;

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
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw3DaoExecuteMsgIndexer {
    fn default() -> Self {
        Cw3DaoExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(INDEXER_KEY)],
        }
    }
}

impl Indexer for Cw3DaoExecuteMsgIndexer {
    type MessageType = Cw3DaoExecuteMsg;
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

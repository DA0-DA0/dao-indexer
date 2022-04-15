use super::indexer::Indexer;
use super::indexer_registry::RegistryKey;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;

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
            registry_keys: vec![RegistryKey::new(INDEXER_KEY.to_string())],
        }
    }
}

impl Indexer for Cw3DaoExecuteMsgIndexer {
    type MessageType = Cw3DaoExecuteMsg;
    fn id(&self) -> String {
        INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> Box<dyn Iterator<Item = &RegistryKey> + '_> {
        Box::new(self.registry_keys.iter())
    }
    fn root_keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        // Box::new(ROOT_KEYS.iter().copied())
        Box::new(ROOT_KEYS.into_iter())
    }
}

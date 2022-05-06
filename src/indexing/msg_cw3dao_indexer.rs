use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::RegistryKey;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;

const EXECUTE_MSG_INDEXER_KEY: &str = "Cw3DaoExecuteMsg";
static EXECUTE_MSG_ROOT_KEYS: [&str; 9] = [
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

const INSTANTIATE_MSG_INDEXER_KEY: &str = "Cw3DaoInstantiateMsg";
static INSTANTIATE_MSG_ROOT_KEYS: [&str; 11] = [
        // The name of the DAO.
        "name",
        // A description of the DAO.
        "description",
        // Set an existing governance token or launch a new one
        "gov_token",
        // Set an existing staking contract or instantiate an new one
        "staking_contract",
        // Voting params configuration
        "threshold",
        // The amount of time a proposal can be voted on before expiring
        "max_voting_period",
        // Deposit required to make a proposal
        "proposal_deposit_amount",
        // Refund a proposal if it is rejected
        "refund_failed_proposals",
        // Optional Image URL that is used by the contract
        "image_url",
        "only_members_execute",
        "automatically_add_cw20s",
];

pub struct Cw3DaoExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw3DaoExecuteMsgIndexer {
    fn default() -> Self {
        Cw3DaoExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(EXECUTE_MSG_INDEXER_KEY.to_string())],
        }
    }
}

impl Indexer for Cw3DaoExecuteMsgIndexer {
    type MessageType = Cw3DaoExecuteMsg;
    fn id(&self) -> String {
        EXECUTE_MSG_INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter(EXECUTE_MSG_ROOT_KEYS.into_iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter([].into_iter())
    }
}

pub struct Cw3DaoInstantiateMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw3DaoInstantiateMsgIndexer {
    fn default() -> Self {
        Cw3DaoInstantiateMsgIndexer {
            registry_keys: vec![RegistryKey::new(INSTANTIATE_MSG_INDEXER_KEY.to_string())],
        }
    }
}

impl Indexer for Cw3DaoInstantiateMsgIndexer {
    type MessageType = Cw3DaoInstantiateMsg;
    fn id(&self) -> String {
        INSTANTIATE_MSG_INDEXER_KEY.to_string()
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter(INSTANTIATE_MSG_ROOT_KEYS.into_iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter([].into_iter())
    }
}

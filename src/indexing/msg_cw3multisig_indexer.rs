use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::RegistryKey;
use cw3_multisig::msg::ExecuteMsg as Cw3MultisigExecuteMsg;
use cw3_multisig::msg::InstantiateMsg as Cw3MultisigInstantiateMsg;

const EXECUTE_MSG_INDEXER_KEY: &str = "Cw3MultisigExecuteMsg";
static EXECUTE_MSG_ROOT_KEYS: [&str; 7] = [
    "propose",
    "vote",
    "execute",
    "close",
    "member_changed_hook",
    "update_config",
    "update_cw20_token_list",
];

const INSTANTIATE_MSG_INDEXER_KEY: &str = "Cw3MultisigInstantiateMsg";
static INSTANTIATE_MSG_ROOT_KEYS: [&str; 6] = [
    // The name of the multisig.
    "name",
    // A description of the multisig.
    "description",
    // List of voters that will be used to create a new cw4-group contract
    "group",
    "threshold",
    "max_voting_period",
    "image_url",
];
static INSTANTIATE_MSG_REQUIRED_ROOT_KEYS: [&str; 1] = ["group"];

pub struct Cw3MultisigExecuteMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw3MultisigExecuteMsgIndexer {
    fn default() -> Self {
        Cw3MultisigExecuteMsgIndexer {
            registry_keys: vec![RegistryKey::new(EXECUTE_MSG_INDEXER_KEY.to_string())],
        }
    }
}

// TODO: not if the message is just a vote:
// "{"vote":{"proposal_id":2,"vote":"yes"}}"
impl Indexer for Cw3MultisigExecuteMsgIndexer {
    type MessageType = Cw3MultisigExecuteMsg;
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

pub struct Cw3MultisigInstantiateMsgIndexer {
    registry_keys: Vec<RegistryKey>,
}

impl Default for Cw3MultisigInstantiateMsgIndexer {
    fn default() -> Self {
        Cw3MultisigInstantiateMsgIndexer {
            registry_keys: vec![RegistryKey::new(INSTANTIATE_MSG_INDEXER_KEY.to_string())],
        }
    }
}

impl Indexer for Cw3MultisigInstantiateMsgIndexer {
    type MessageType = Cw3MultisigInstantiateMsg;
    fn id(&self) -> String {
        INSTANTIATE_MSG_INDEXER_KEY.to_string()
    }
    fn has_required_root_keys(&self) -> bool {
        true
    }
    fn registry_keys(&self) -> RegistryKeysType {
        registry_keys_from_iter(self.registry_keys.iter())
    }
    fn root_keys(&self) -> RootKeysType {
        root_keys_from_iter(INSTANTIATE_MSG_ROOT_KEYS.into_iter())
    }
    fn required_root_keys(&self) -> RootKeysType {
        root_keys_from_iter(INSTANTIATE_MSG_REQUIRED_ROOT_KEYS.into_iter())
    }
}

use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer::{
    registry_keys_from_iter, root_keys_from_iter, Indexer, RegistryKeysType, RootKeysType,
};
use super::indexer_registry::{IndexerRegistry, RegistryKey};
use crate::util::contract_util::get_contract_addresses;
use crate::util::dao::{get_single_event_item, get_tx_height_from_events, insert_dao};
use crate::util::gov_token::gov_token_from_msg;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
use log::{debug, error};
use serde_json::Value;

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

    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        if msg.get("threshold").is_some()
            && msg
                .get("threshold")
                .unwrap()
                .get("absolute_count")
                .is_some()
        {
            debug!("msg_cw3dao_indexer ignoring multisig\n{:#?}", msg);
            return None;
        }
        self.first_matching_key(msg)
    }

    // Indexes a message and its transaction events
    fn index<'a>(
        &'a self,
        // The registry of indexers
        registry: &'a IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        events: &'a EventMap,
        // Generic serde-parsed value dictionary
        msg_dictionary: &'a Value,
        // The decoded string value of the message
        msg_str: &'a str,
    ) -> anyhow::Result<()> {
        match serde_json::from_str::<Self::MessageType>(msg_str) {
            Ok(msg) => msg.index_message(registry, events),
            Err(_e) => match serde_json::from_str::<Cw3DaoInstantiateMsg25>(msg_str) {
                Ok(msg) => msg.index_message(registry, events),
                Err(e) => {
                    error!("{} Error deserializing {:#?}", self.id(), e);
                    self.index_message_dictionary(registry, events, msg_dictionary, msg_str)
                }
            },
        }
    }

    // This is the fallback indexer if all the direct deserialization has failed.
    fn index_message_dictionary<'a>(
        &'a self,
        registry: &'a super::indexer_registry::IndexerRegistry,
        events: &'a super::event_map::EventMap,
        msg_dictionary: &'a Value,
        _msg_str: &'a str,
    ) -> anyhow::Result<()> {
        let contract_addresses = get_contract_addresses(events);
        let tx_height = get_tx_height_from_events(events);
        let mut image_url = None;
        let image_url_str = get_single_event_item(events, "image_url", "").to_string();
        if !image_url_str.is_empty() {
            image_url = Some(&image_url_str)
        }
        let mut dao_name = &"".to_string();
        let mut dao_description = &"".to_string();
        if let Some(Value::String(val)) = msg_dictionary.get("name") {
            dao_name = val;
        }
        if let Some(Value::String(val)) = msg_dictionary.get("description") {
            dao_description = val;
        }
        // TODO: max_voting_period, proposal_deposit_amount, refund_failed_proposals, threshold,
        if let Some(gov_token) = gov_token_from_msg(msg_dictionary) {
            insert_dao(
                registry,
                dao_name,
                dao_description,
                &gov_token,
                image_url,
                &contract_addresses,
                Some(&tx_height),
            )
        } else {
            error!("Could not parse GovTokenMsg from {:#?}", msg_dictionary);
            Ok(())
        }
    }
}

#[test]
fn test_schema_types() {
    use log::warn;
    use schemars::schema::{InstanceType, SingleOrVec};
    use schemars::schema_for;
    let schema3 = schema_for!(Cw3DaoInstantiateMsg);
    // let schema25 = schema_for!(Cw3DaoInstantiateMsg25);

    // if let Some(objects) = schema3.schema.object {
    //     println!("objects:\n{:#?}", objects);
    // }

    let instance_type = &(schema3.schema.instance_type.unwrap());
    match instance_type {
        SingleOrVec::Single(itype) => {
            println!("itype {:?}", itype);
            match itype.as_ref() {
                &InstanceType::Object => {
                    // println!("Yes, it's an object, properties:\n{:#?}", &(schema3.schema.object.unwrap().properties.keys().clone()));
                    let properties = &(schema3.schema.object.unwrap().properties);
                    let mut required_roots = vec![];
                    for (property_name, schema) in properties {
                        println!("property_name: {}", property_name);
                        match schema {
                            schemars::schema::Schema::Object(schema) => {
                                match &schema.instance_type {
                                    Some(type_instance) =>
                                    {
                                        match type_instance {
                                            SingleOrVec::Single(single_val) => {
                                                println!("Single value");
                                                required_roots.push(property_name);
                                                match *single_val.as_ref() {
                                                    InstanceType::Boolean => {
                                                        println!("Boolean");
                                                    }
                                                    InstanceType::String => {
                                                        println!("String");
                                                    }
                                                    _ => {
                                                        println!("Not handled");
                                                    }
                                                }
                                            }
                                            SingleOrVec::Vec(vec_val) => {
                                                println!("Vec value {:#?}", vec_val);
                                            }
                                        }
                                    }
                                    None => {
                                        println!("{} has no instance_type", property_name);
                                        required_roots.push(property_name);
                                    }
                                    _ => {
                                        println!(
                                            "Not dealing with instance type {:?} yet",
                                            schema.instance_type
                                        );
                                    }
                                }
                            }
                            _ => {
                                warn!("Not an object type: {:#?}", schema);
                            }
                        }
                    }
                    println!("required roots: {:#?}", required_roots);
                    // println!("property details:\n{:#?}", properties);
                }
                _ => {
                    println!("god only knows");
                }
            }
        }
        _ => {
            println!("not object");
        }
    }
    // println!(
    //     "schema3: \n{}\nschema25:\n{}",
    //     serde_json::to_string_pretty(&schema3).unwrap(),
    //     serde_json::to_string_pretty(&schema25).unwrap()
    // );
}

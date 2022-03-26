use super::index::Index;
use super::indexer::Indexer;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
pub use cw20::Cw20ExecuteMsg;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use diesel::pg::PgConnection;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;

struct MsgExecuteContractIndexer {
    my_registry_keys: Vec<String>,
}

impl Default for MsgExecuteContractIndexer {
    fn default() -> Self {
        MsgExecuteContractIndexer {
            my_registry_keys: vec![
                "stakecw20_execute".to_string(),
                "Cw20ExecuteMsg".to_string(),
                "cw3dao_execute".to_string(),
            ],
        }
    }
}

impl Indexer for MsgExecuteContractIndexer {
    fn index(
        &self,
        db: &PgConnection,
        events: &Option<BTreeMap<String, Vec<String>>>,
        msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn id(&self) -> String {
        "MsgExecuteContractIndexer".to_string()
    }
    fn registry_keys(&self) -> std::slice::Iter<String> {
        self.my_registry_keys.iter()
    }
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<String> {
        let cw20_root_keys = vec![
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
        for key in cw20_root_keys {
            if msg.get(key).is_some() {
                return Some("Cw20ExecuteMsg".to_string());
            }
        }
        None
    }
}

impl Index for MsgExecuteContract {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg_str = String::from_utf8(self.msg.clone())?;
        let msg_val: Value = serde_json::from_str(&msg_str)?;
        registry.index(events, &msg_val, &msg_str)?;
        let mut errors = vec![];
        match serde_json::from_str::<Cw3DaoExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(registry, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<StakeCw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(registry, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<Cw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(registry, events);
            }
            Err(e) => {
                errors.push(e);
            }
        }
        Err(Box::from(format!(
            "could not interpret execute msg, got errors:\n{:?}",
            errors
        )))
    }
}

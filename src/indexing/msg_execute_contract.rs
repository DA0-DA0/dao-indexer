use super::index::Index;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
pub use cw20::Cw20ExecuteMsg;
use serde_json::Value;
use std::collections::BTreeMap;

impl Index for MsgExecuteContract {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg_str = String::from_utf8(self.msg.clone())?;
        let msg_val: Value = serde_json::from_str(&msg_str)?;
        registry.index(events, &msg_val, &msg_str)
        // let mut errors = vec![];
        // // match serde_json::from_str::<Cw3DaoExecuteMsg>(&msg_str) {
        // //     Ok(execute_contract) => {
        // //         return execute_contract.index(registry, events);
        // //     }
        // //     Err(e) => {
        // //         errors.push(e);
        // //     }
        // // };
        // match serde_json::from_str::<StakeCw20ExecuteMsg>(&msg_str) {
        //     Ok(execute_contract) => {
        //         return execute_contract.index(registry, events);
        //     }
        //     Err(e) => {
        //         errors.push(e);
        //     }
        // };
        // // match serde_json::from_str::<Cw20ExecuteMsg>(&msg_str) {
        // //     Ok(execute_contract) => {
        // //         return execute_contract.index(registry, events);
        // //     }
        // //     Err(e) => {
        // //         errors.push(e);
        // //     }
        // // }
        // Err(Box::from(format!(
        //     "could not interpret execute msg, got errors:\n{:?}",
        //     errors
        // )))
    }
}

use super::index::Index;
use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
pub use cw20::Cw20ExecuteMsg;
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use diesel::pg::PgConnection;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;

impl Index for MsgExecuteContract {
  fn index(
    &self,
    db: &PgConnection,
    events: &Option<BTreeMap<String, Vec<String>>>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let msg_str = String::from_utf8(self.msg.clone())?;
    let mut errors = vec![];
    match serde_json::from_str::<Cw3DaoExecuteMsg>(&msg_str) {
      Ok(execute_contract) => {
        return execute_contract.index(db, events);
      }
      Err(e) => {
        errors.push(e);
      }
    };
    match serde_json::from_str::<StakeCw20ExecuteMsg>(&msg_str) {
      Ok(execute_contract) => {
        return execute_contract.index(db, events);
      }
      Err(e) => {
        errors.push(e);
      }
    };
    match serde_json::from_str::<Cw20ExecuteMsg>(&msg_str) {
      Ok(execute_contract) => {
        return execute_contract.index(db, events);
      }
      Err(e) => {
        errors.push(e);
      }
    }
    Err(Box::from(format!("could not interpret execute msg, got errors:\n{:?}", errors)))
  }
}
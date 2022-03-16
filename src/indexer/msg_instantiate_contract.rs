use super::index::Index;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};
use bigdecimal::BigDecimal;
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use diesel::pg::PgConnection;
use std::collections::BTreeMap;
use std::str::FromStr;
use crate::util::insert_dao::insert_dao;

impl Index for MsgInstantiateContract {
  fn index(
    &self,
    db: &PgConnection,
    events: &Option<BTreeMap<String, Vec<String>>>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let contract_addresses = get_contract_addresses(events);
    let dao_address = contract_addresses.dao_address.as_ref().unwrap();
    let staking_contract_address = contract_addresses
      .staking_contract_address
      .as_ref()
      .unwrap();
    let mut tx_height_opt = None;
    if let Some(event_map) = events {
      let tx_height_strings = event_map.get("tx.height").unwrap();
      if !tx_height_strings.is_empty() {
        let tx_height_str = &tx_height_strings[0];
        tx_height_opt = Some(BigDecimal::from_str(tx_height_str).unwrap());
      }
    }
    let tx_height: BigDecimal;
    if let Some(height) = tx_height_opt {
      tx_height = height;
    } else {
      tx_height = BigDecimal::from_str("0").unwrap();
    }

    let contract_model =
      NewContract::from_msg(dao_address, staking_contract_address, &tx_height, self);
    insert_contract(db, &contract_model);
    let msg_str = String::from_utf8(self.msg.clone()).unwrap();
    let instantiate_dao = serde_json::from_str::<Cw3DaoInstantiateMsg>(&msg_str)?;
    insert_dao(db, &instantiate_dao, &contract_addresses, Some(&tx_height))
  }
}

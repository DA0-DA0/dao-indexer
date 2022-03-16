use std::collections::BTreeMap;
use crate::db::models::NewContract;
use diesel::pg::PgConnection;
use cw3_dao::msg::{
  InstantiateMsg as Cw3DaoInstantiateMsg,
};
use bigdecimal::BigDecimal;

#[derive(Debug)]
pub struct ContractAddresses {
    pub dao_address: Option<String>,
    pub cw20_address: Option<String>,
    pub staking_contract_address: Option<String>,
}

pub fn get_contract_addresses(events: &Option<BTreeMap<String, Vec<String>>>) -> ContractAddresses {
  let mut dao_address = None;
  let mut cw20_address = None;
  let mut staking_contract_address = None;
  if let Some(transaction_events) = events {
      if let Some(addr) = transaction_events.get("instantiate._contract_address") {
          // 0: DAO
          // 1: cw20
          // 2: staking contract
          // But if you use an existing token, you'll just get
          // DAO/staking contract
          dao_address = Some(addr[0].clone());
          cw20_address = Some(addr[1].clone());
          staking_contract_address = Some(addr[2].clone());
      }
  }
  ContractAddresses {
      dao_address,
      cw20_address,
      staking_contract_address,
  }
}

pub fn insert_dao(
  db: &PgConnection,
  instantiate_dao: &Cw3DaoInstantiateMsg,
  contract_addr: &ContractAddresses,
  height: Option<&BigDecimal>,
) {
  use crate::db::schema::dao::dsl::*;

  let dao_address = contract_addr.dao_address.as_ref().unwrap();

  let inserted_token_id: i32 =
      insert_gov_token(db, &instantiate_dao.gov_token, contract_addr, height).unwrap();

  let dao_model = NewDao::from_msg(
      dao_address,
      contract_addr.staking_contract_address.as_ref().unwrap(),
      inserted_token_id,
      instantiate_dao,
  );

  diesel::insert_into(dao)
      .values(dao_model)
      .execute(db)
      .expect("Error saving dao");
}

pub fn insert_contract(db: &PgConnection, contract_model: &NewContract) {
  use crate::db::schema::contracts::dsl::*;
  diesel::insert_into(contracts)
      .values(contract_model)
      .execute(db)
      .expect("Error saving new post");
}


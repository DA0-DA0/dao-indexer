use crate::db::models::NewContract;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::BTreeMap;

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

pub fn insert_contract(db: &PgConnection, contract_model: &NewContract) {
  use crate::db::schema::contracts::dsl::*;
  diesel::insert_into(contracts)
    .values(contract_model)
    .execute(db)
    .expect("Error saving new post");
}
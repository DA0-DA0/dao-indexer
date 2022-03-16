use bigdecimal::BigDecimal;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use cw20_base::msg::InstantiateMarketingInfo;
use cw3_dao::msg::{
    ExecuteMsg as Cw3DaoExecuteMsg, GovTokenMsg, InstantiateMsg as Cw3DaoInstantiateMsg,
};
use crate::db::connection::establish_connection;
use crate::db::models::{Cw20, Dao, NewContract, NewDao, NewGovToken};
use crate::historical_parser::block_synchronizer;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;
use std::str::FromStr;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use dotenv::dotenv;
use std::env;
use super::contract_util::ContractAddresses;
use super::insert_marketing_info::insert_marketing_info;

pub fn insert_gov_token(
  db: &PgConnection,
  token_msg: &GovTokenMsg,
  contract_addresses: &ContractAddresses,
  height: Option<&BigDecimal>,
) -> QueryResult<i32> {
  use crate::db::schema::gov_token::dsl::*;
  let result: QueryResult<i32>;
  match token_msg {
      GovTokenMsg::InstantiateNewCw20 {
          msg,
          initial_dao_balance,
          ..
      } => {
          let mut marketing_record_id: Option<i32> = None;
          if let Some(marketing) = &msg.marketing {
              marketing_record_id = Some(insert_marketing_info(db, marketing).unwrap());
          }
          let cw20_address = contract_addresses.cw20_address.as_ref().unwrap();
          let token_model = NewGovToken::from_msg(cw20_address, marketing_record_id, msg);
          result = diesel::insert_into(gov_token)
              .values(token_model)
              .returning(id)
              .get_result(db);
          let dao_address = contract_addresses.dao_address.as_ref().unwrap();
          let amount;
          if let Some(balance) = initial_dao_balance {
              amount = *balance;
          } else {
              amount = Uint128::from(0u128);
          }
          let balance_update = Cw20Coin {
              address: dao_address.to_string(),
              amount,
          };
          let initial_update_result = update_balance(
              db,
              height,
              cw20_address,
              dao_address, // As the minter the DAO is also the sender for its own initial balance (???)
              &balance_update,
          );
          if let Err(e) = initial_update_result {
              eprintln!("error updating initial balance {}", e);
          }

          if let Ok(_token_id) = result {
              // This handles the initial token distributions but not the treasury.
              for balance in &msg.initial_balances {
                  if let Err(e) = update_balance(db, height, cw20_address, dao_address, balance) {
                      eprintln!("{}", e);
                  }
              }
          }
      }
      GovTokenMsg::UseExistingCw20 {
          addr,
          stake_contract_code_id,
          label,
          unstaking_duration,
      } => {
          println!("TODO: Use existing cw20 addr: {}, stake_contract_code_id: {}, label: {}, unstaking_duration: {:?}", addr, stake_contract_code_id, label, unstaking_duration);
          result = Ok(0);
      }
  };
  result
}


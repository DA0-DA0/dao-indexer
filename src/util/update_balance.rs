pub use cw20::Cw20ExecuteMsg;
use bigdecimal::BigDecimal;
use cw20::Cw20Coin;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::str::FromStr;
use std::collections::BTreeMap;
use cosmwasm_std::Uint128;
use super::gov_token::get_gov_token;

pub fn update_balance(
  db: &PgConnection,
  tx_height: Option<&BigDecimal>,
  token_addr: &str,
  token_sender_address: &str,
  balance_update: &Cw20Coin,
) -> QueryResult<usize> {
  use crate::db::schema::cw20_transactions::dsl::*;
  let amount_converted: BigDecimal = BigDecimal::from(balance_update.amount.u128() as i64);
  let transaction_height: BigDecimal;
  if let Some(tx_height_value) = tx_height {
      transaction_height = tx_height_value.clone();
  } else {
      transaction_height = BigDecimal::from_str("0").unwrap();
  }
  diesel::insert_into(cw20_transactions)
      .values((
          cw20_address.eq(token_addr),
          sender_address.eq(token_sender_address),
          recipient_address.eq(&balance_update.address),
          height.eq(&transaction_height),
          amount.eq(amount_converted),
      ))
      .execute(db)
}

pub fn update_balance_from_events(
  db: &PgConnection,
  i: usize,
  event_map: &BTreeMap<String, Vec<String>>,
) -> QueryResult<usize> {
  let tx_height_string = &event_map.get("tx.height").unwrap()[0];
  let tx_height = BigDecimal::from_str(tx_height_string).unwrap();
  let amount = &event_map.get("wasm.amount").unwrap()[i];
  let receiver = &event_map.get("wasm.to").unwrap()[i];
  let sender = &event_map.get("wasm.sender").unwrap()[0];
  let from = &event_map.get("wasm.from").unwrap()[0]; // DAO address
  let gov_token = get_gov_token(db, from).unwrap();
  let balance_update = Cw20Coin {
      address: receiver.clone(),
      amount: Uint128::from_str(amount).unwrap(),
  };
  update_balance(
      db,
      Some(&tx_height),
      &gov_token.address,
      sender,
      &balance_update,
  )
}
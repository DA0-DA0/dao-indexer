pub use cw20::Cw20ExecuteMsg;
use bigdecimal::BigDecimal;
use cw20::Cw20Coin;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::str::FromStr;

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

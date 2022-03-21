use super::gov_token::get_gov_token;
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::BTreeMap;
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
    let from;
    match event_map.get("wasm.from") {
        Some(wasm_from) => {
            from = wasm_from[0].to_string();
        }
        _ => {
            from = "".to_string();
        }
    }
    if !from.is_empty() {
        let gov_token = get_gov_token(db, &from).unwrap();
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
    } else {
        eprintln!("No 'wasm.from' value found in event map");
        Ok(0)
    }
}

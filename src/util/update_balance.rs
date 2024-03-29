use super::gov_token::get_gov_token_address;
use crate::indexing::indexer_registry::IndexerRegistry;
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
pub use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::error;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use std::str::FromStr;

#[test]
fn test_big_decimal() {
    use bigdecimal::ToPrimitive;
    use num_bigint::ToBigInt;
    let big_u128: u128 = u128::MAX - 10;
    let converted = BigDecimal::from(BigInt::from(big_u128));
    let converted_back = converted.to_bigint().unwrap().to_u128().unwrap();
    dbg!(big_u128);
    dbg!(converted);
    assert_eq!(big_u128, converted_back);
}

pub fn update_balance<'a>(
    db: impl Into<&'a PgConnection>, // TODO(gavin.doughtie): also below
    tx_height: Option<&BigDecimal>,
    token_addr: &str,
    token_sender_address: &str,
    recipient: &str,
    balance_amount: u128,
) -> QueryResult<usize> {
    use crate::db::schema::cw20_transactions::dsl::*;
    let amount_converted: BigDecimal = BigDecimal::from(BigInt::from(balance_amount));

    let transaction_height: BigDecimal = if let Some(tx_height_value) = tx_height {
        tx_height_value.clone()
    } else {
        BigDecimal::default()
    };

    diesel::insert_into(cw20_transactions)
        .values((
            cw20_address.eq(token_addr),
            sender_address.eq(token_sender_address),
            recipient_address.eq(recipient),
            height.eq(&transaction_height),
            amount.eq(amount_converted),
        ))
        .execute(db.into())
}

pub fn update_balance_from_events(
    db: &IndexerRegistry,
    i: usize,
    event_map: &BTreeMap<String, Vec<String>>,
) -> QueryResult<usize> {
    let tx_height_string = &event_map.get("tx.height").unwrap()[0];
    let tx_height = BigDecimal::from_str(tx_height_string).unwrap();
    let amount = &event_map.get("wasm.amount").unwrap()[i];
    let receiver = &event_map.get("wasm.to").unwrap()[i];
    let sender = &event_map.get("wasm.sender").unwrap()[0];

    let from = match event_map.get("wasm.from") {
        Some(wasm_from) => wasm_from[0].to_string(),
        _ => "".to_string(),
    };

    if !from.is_empty() {
        let mut parsed_amount: Uint128 = Uint128::new(0);
        match Uint128::from_str(amount) {
            Ok(ok_parsed_amount) => {
                parsed_amount = ok_parsed_amount;
            }
            Err(e) => {
                eprintln!("Error parsing amount: {} {:?}", amount, e);
            }
        }
        if let Some(gov_token_address) = get_gov_token_address(db, &from) {
            update_balance(
                db,
                Some(&tx_height),
                &gov_token_address,
                sender,
                receiver,
                u128::from(parsed_amount),
            )
        } else {
            Ok(0)
        }
    } else {
        error!("No 'wasm.from' value found in event map");
        Ok(0)
    }
}

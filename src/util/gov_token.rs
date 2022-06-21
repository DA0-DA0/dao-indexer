use super::contract_util::ContractAddresses;
use super::dao::get_dao;
use super::insert_marketing_info::insert_marketing_info;
use super::update_balance::update_balance;
use crate::{
    db::models::{Cw20, NewGovToken},
    indexing::indexer_registry::IndexerRegistry,
};
use cw3_dao_2_5::msg::GovTokenMsg as GovTokenMsg25;

use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
pub use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw20_011_1::Cw20Coin as Cw20Coin_11_1;
use cw3_dao::msg::{GovTokenInstantiateMsg, GovTokenMsg};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::{error, warn};
use serde_json::Value;

pub fn cw20_coin_from_value(value_dict: &Value) -> Option<Cw20Coin_11_1> {
    match serde_json::from_value::<Cw20Coin_11_1>(value_dict.clone()) {
        Ok(coin) => {
            return Some(coin);
        }
        Err(e) => {
            error!("Error parsing coin {:#?}, {:#?}", value_dict, e);
        }
    }
    None
}

pub fn gov_token_from_instantiate(value_dict: &Value) -> Option<GovTokenInstantiateMsg> {
    let mut name = "".to_string();
    let mut symbol = "".to_string();
    let mut decimals = 0u8;
    let mut initial_balances = vec![];
    if let Some(name_str) = value_dict.get("name") {
        name = name_str.to_string();
    }

    if let Some(symbol_str) = value_dict.get("symbol") {
        symbol = symbol_str.to_string();
    }

    if let Some(decimals_value) = value_dict.get("decimals") {
        if let Value::Number(decimals_number) = decimals_value {
            decimals = decimals_number.as_u64().unwrap_or_default() as u8;
        } else {
            error!("unable to parse decimals for {:#?}", decimals_value);
        }
    }

    if let Some(Value::Array(cw20s)) = value_dict.get("initial_balances") {
        for cw20 in cw20s {
            if let Some(coin) = cw20_coin_from_value(cw20) {
                initial_balances.push(coin);
            }
        }
    }

    let marketing = None;
    let msg = GovTokenInstantiateMsg {
        name,
        symbol,
        decimals,
        initial_balances,
        marketing,
    };
    Some(msg)
}

pub fn gov_token_from_value(value_dict: &Value) -> Option<GovTokenMsg> {
    if let Ok(gov_token) = serde_json::from_value::<GovTokenMsg>(value_dict.clone()) {
        return Some(gov_token);
    }
    error!("failed to parse a GovTokenMsg from {:#?}", value_dict);
    if let Some(token_dict) = value_dict.get("instantiate_new_cw20") {
        if let Some(instantiate) = gov_token_from_instantiate(token_dict) {
            let msg: GovTokenMsg = GovTokenMsg::InstantiateNewCw20 {
                cw20_code_id: 0,
                label: "".to_string(),
                initial_dao_balance: None,
                msg: instantiate,
            };
            return Some(msg);
        }
    }
    if let Some(token_dict) = value_dict.get("use_existing_cw20") {
        warn!(
            "Should be trying to parse the existing cw20 message: {:#?}",
            token_dict
        );
    }

    None
}

pub fn gov_token_from_msg(value_dict: &Value) -> Option<GovTokenMsg> {
    if let Some(token_dict) = value_dict.get("gov_token") {
        return gov_token_from_value(token_dict);
    }
    None
}

// fn convert_coin_2_5_to_2_1(coin_2_5: &Cw20Coin) -> Cw20Coin_11_1 {
//     Cw20Coin_11_1 {
//         address: coin_2_5.address,
//         amount: coin_2_5.amount,
//     }
// }

fn convert_2_5_to_3(msg: &GovTokenMsg25) -> GovTokenMsg {
    match msg {
        GovTokenMsg25::InstantiateNewCw20 {
            cw20_code_id,
            stake_contract_code_id: _,
            label,
            initial_dao_balance,
            msg,
            unstaking_duration: _,
        } => {
            let mut initial_balances: Vec<Cw20Coin_11_1> = vec![];
            for coin in &msg.initial_balances {
                let converted_coin = Cw20Coin_11_1 {
                    address: coin.address.clone(),
                    amount: coin.amount,
                };
                initial_balances.push(converted_coin);
            }
            let msg = GovTokenInstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol.clone(),
                decimals: msg.decimals,
                initial_balances,
                marketing: msg.marketing.clone(),
            };
            GovTokenMsg::InstantiateNewCw20 {
                cw20_code_id: *cw20_code_id,
                label: label.clone(),
                initial_dao_balance: *initial_dao_balance,
                msg,
            }
        }
        GovTokenMsg25::UseExistingCw20 {
            addr,
            label,
            stake_contract_code_id: _,
            unstaking_duration: _,
        } => GovTokenMsg::UseExistingCw20 {
            addr: addr.clone(),
            label: label.clone(),
        },
    }
}

pub fn insert_gov_token25(
    db: &IndexerRegistry,
    token_msg: &GovTokenMsg25,
    contract_addresses: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> QueryResult<i32> {
    insert_gov_token(db, &convert_2_5_to_3(token_msg), contract_addresses, height)
}

pub fn insert_gov_token(
    db: &IndexerRegistry,
    token_msg: &GovTokenMsg,
    contract_addresses: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> QueryResult<i32> {
    use crate::db::schema::gov_token::dsl::*;
    match token_msg {
        GovTokenMsg::InstantiateNewCw20 {
            msg,
            initial_dao_balance,
            ..
        } => {
            let mut marketing_record_id: Option<i32> = None;
            if let Some(marketing) = &msg.marketing {
                marketing_record_id = Some(
                    insert_marketing_info(
                        db,
                        marketing.project.as_deref().unwrap_or_default(),
                        marketing.description.as_deref().unwrap_or_default(),
                        marketing.marketing.as_deref().unwrap_or_default(),
                    )
                    .unwrap(),
                );
            }
            let cw20_address = contract_addresses.cw20_address.as_ref().unwrap();
            let token_model = NewGovToken::from_msg(cw20_address, marketing_record_id, msg);
            let _ = diesel::insert_into(gov_token)
                .values(token_model)
                .execute(db as &PgConnection);
            let dao_address = contract_addresses.contract_address.as_ref().unwrap();
            let amount = if let Some(balance) = initial_dao_balance {
                *balance
            } else {
                Uint128::from(0u128)
            };
            let initial_update_result = update_balance(
                db,
                height,
                cw20_address,
                dao_address,
                dao_address, // As the minter the DAO is also the sender for its own initial balance (???)
                u128::from(amount),
            );
            if let Err(e) = initial_update_result {
                error!("error updating initial balance {}", e);
            } else {
                // This handles the initial token distributions but not the treasury.
                for balance in &msg.initial_balances {
                    let amount = balance.amount;
                    let recipient = &balance.address;
                    if let Err(e) = update_balance(
                        db,
                        height,
                        cw20_address,
                        dao_address,
                        recipient,
                        u128::from(amount),
                    ) {
                        error!("Error updating balance {:?}", e);
                    }
                }
            }
        }
        GovTokenMsg::UseExistingCw20 {
            addr,
            // stake_contract_code_id,
            label,
            // unstaking_duration,
            ..
        } => {
            warn!("TODO: Use existing cw20 addr: {}, label: {},", addr, label);
        }
    };
    //}
    Ok(0)
}

pub fn get_gov_token_address(db: &PgConnection, dao_address: &str) -> Option<String> {
    match get_dao(db, dao_address) {
        Ok(dao) => dao.gov_token_address,
        Err(e) => {
            error!("Error getting dao for address '{}':\n{}", dao_address, e);
            None
        }
    }
}

pub fn get_gov_token(db: &PgConnection, dao_address: &str) -> diesel::QueryResult<Option<Cw20>> {
    use crate::db::schema::gov_token::dsl::*;
    match get_dao(db, dao_address) {
        Ok(dao) => {
            if let Some(gov_token_address) = dao.gov_token_address {
                gov_token
                    .filter(address.eq(gov_token_address))
                    .first(db)
                    .optional()
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            error!("Error getting dao for address '{}'", dao_address);
            Err(e)
        }
    }
}

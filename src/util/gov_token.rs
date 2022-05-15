use super::contract_util::ContractAddresses;
use super::dao::get_dao;
use super::insert_marketing_info::insert_marketing_info;
use super::update_balance::update_balance;
use crate::{
    db::models::{Cw20, NewGovToken},
    indexing::indexer_registry::IndexerRegistry,
};
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
pub use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw3_dao::msg::{GovTokenMsg, GovTokenInstantiateMsg};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::{error, warn};
use serde_json::Value;

pub fn cw20_coin_from_value(value_dict: &Value) -> Option<Cw20Coin> {
    None
}

pub fn gov_token_from_instantiate(value_dict: &Value) -> Option<GovTokenInstantiateMsg> {
    let name = value_dict.get::<&str>("name").unwrap_or_default().to_string();
    let msg: GovTokenInstantiateMsg = {
        name,
        pub symbol: String,
        pub decimals: u8,
        pub initial_balances: Vec<Cw20Coin>,
        pub marketing: Option<InstantiateMarketingInfo>,    
    };
    Some(msg)
}

pub fn gov_token_from_value(value_dict: &Value) -> Option<GovTokenMsg> {
    if let Some(token_dict) = value_dict.get("instantiate_new_cw20") {
        if let Some(instantiate) = gov_token_from_instantiate(token_dict) {
            let msg: GovTokenMsg = GovTokenMsg::InstantiateNewCw20 {
                cw20_code_id: 0,
                label: "".to_string(),
                initial_dao_balance: None,
                msg: instantiate
            };
            return Some(msg);
        }
    }

    None
}

pub fn gov_token_from_msg(value_dict: &Value) -> Option<GovTokenMsg> {
    if let Some(token_dict) = value_dict.get("gov_token") {
        return gov_token_from_value(token_dict);
    }
    None
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
            let amount;
            if let Some(balance) = initial_dao_balance {
                amount = *balance;
            } else {
                amount = Uint128::from(0u128);
            }
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

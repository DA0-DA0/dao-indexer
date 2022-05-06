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
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use cw3_dao::msg::GovTokenMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::{error, warn};

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
                marketing_record_id = Some(insert_marketing_info(db, marketing).unwrap());
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
                error!("error updating initial balance {}", e);
            } else {
                // This handles the initial token distributions but not the treasury.
                for balance in &msg.initial_balances {
                    if let Err(e) = update_balance(db, height, cw20_address, dao_address, balance) {
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

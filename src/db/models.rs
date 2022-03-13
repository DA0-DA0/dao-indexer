use super::schema::{contracts, cw20_balances, gov_token};
use bigdecimal::BigDecimal; // Has to match diesel's version!
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::GovTokenInstantiateMsg;
use diesel::sql_types::{BigInt, Jsonb, Numeric, Text};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Insertable)]
#[table_name = "contracts"]
pub struct NewContract<'a> {
    pub address: &'a str,
    pub staking_contract_address: &'a str,
    pub code_id: i64,
    pub creator: &'a str,
    pub admin: &'a str,
    pub label: &'a str,
    pub creation_time: &'a str,
    pub height: &'a BigDecimal,
}

impl<'a> NewContract<'a> {
    pub fn from_msg(
        dao_address: &'a str,
        staking_contract_address: &'a str,
        tx_height: &'a BigDecimal,
        msg: &'a MsgInstantiateContract,
    ) -> NewContract<'a> {
        let code_id: i64 = msg.code_id as i64;
        NewContract {
            address: dao_address,
            staking_contract_address,
            admin: &msg.admin,
            code_id,
            creator: &msg.sender,
            label: &msg.label,
            creation_time: "",
            height: tx_height,
        }
    }
}

// TODO(gavin.doughtie): These are out of date and we're just
// using the diesel fields directly right now, but it's going
// to be better to move to these structs in the long run so
// leaving them here for reference.

#[derive(Queryable)]
pub struct Contract {
    pub address: Text,
    pub staking_contract_address: Text,
    pub code_id: BigInt,
    pub creator: Text,
    pub admin: Text,
    pub label: Text,
    pub creation_time: Text,
    pub height: Numeric,
    pub json: Jsonb,
}

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct Dao {
    pub id: i32,
    pub contract_adress: String,
    pub staking_contract_adress: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub gov_token_id: i32,
}

#[derive(Insertable)]
#[table_name = "cw20_balances"]
pub struct NewCw20Balance<'a> {
    pub address: &'a str,
    pub token: &'a str,
    pub balance: &'a BigDecimal,
}

#[derive(Queryable)]
pub struct Cw20Balance {
    pub address: Text,
    pub token: Text,
    pub code_id: BigInt,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cw20Msg {
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
}

// Data from the gov_token table:
#[derive(Serialize, Deserialize, Debug, Queryable)]
pub struct Cw20 {
    pub id: i32,
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: Option<i32>,
    pub marketing_id: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "gov_token"]
pub struct NewGovToken<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub symbol: &'a str,
    pub decimals: i32,
    pub marketing_id: Option<i32>,
}

impl<'a> NewGovToken<'a> {
    pub fn from_msg(
        cw20_address: &'a str,
        marketing_id: Option<i32>,
        msg: &'a GovTokenInstantiateMsg,
    ) -> NewGovToken<'a> {
        NewGovToken {
            name: &msg.name,
            address: cw20_address,
            symbol: &msg.symbol,
            decimals: msg.decimals as i32,
            marketing_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovToken {
    pub instantiate_new_cw20: Cw20,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewDao {
    pub description: String,
    pub gov_token: GovToken,
}

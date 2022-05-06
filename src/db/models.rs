use super::schema::block;
use super::schema::{contracts, cw20_balances, dao, gov_token};
use bigdecimal::BigDecimal; // Has to match diesel's version!
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::GovTokenInstantiateMsg;
use diesel::sql_types::{BigInt, Jsonb, Numeric, Text};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Insertable, Debug)]
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
        address: &'a str,
        staking_contract_address: &'a str,
        tx_height: &'a BigDecimal,
        msg: &'a MsgInstantiateContract,
    ) -> NewContract<'a> {
        let code_id: i64 = msg.code_id as i64;
        NewContract {
            address,
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
    pub contract_adress: String,
    pub staking_contract_adress: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub gov_token_address: Option<String>,
}

#[derive(Insertable)]
#[table_name = "dao"]
pub struct NewDao<'a> {
    pub contract_address: &'a str,
    pub staking_contract_address: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub image_url: Option<&'a String>,
    pub gov_token_address: Option<&'a String>,
}

impl<'a> NewDao<'a> {
    pub fn new(
        contract_address: &'a str,
        description: &'a str,
        gov_token_address: Option<&'a String>,
        image_url: Option<&'a String>,
        name: &'a str,
        staking_contract_address: &'a str,
    ) -> NewDao<'a> {
        NewDao {
            contract_address,
            description,
            gov_token_address,
            image_url,
            name,
            staking_contract_address,
        }
    }
}

#[derive(Insertable)]
#[table_name = "cw20_balances"]
pub struct NewCw20Balance<'a> {
    pub address: &'a str,
    pub token: &'a str,
    pub balance: BigDecimal,
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

#[derive(Insertable)]
#[table_name = "block"]
pub struct NewBlock<'a> {
    pub height: i64,
    pub hash: &'a str,
    pub num_txs: i64,
    // pub total_gas: i64,
    // pub proposer_address: &'a str,
}

impl<'a> NewBlock<'a> {
    pub fn from_block_response(hash: &'a str, block: &'a tendermint::block::Block) -> NewBlock<'a> {
        NewBlock {
            height: block.header.height.value() as i64,
            hash,
            num_txs: block.data.iter().len() as i64,
        }
    }
}

#[derive(Queryable)]
pub struct Block {
    pub height: i64,
    pub hash: String,
    pub num_txs: Option<i64>,
    // pub total_gas: BigInt,
    // pub proposer_address: Text
}

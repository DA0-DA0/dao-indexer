use super::schema::{contracts, cw20_balances};
use diesel::sql_types::{Text, BigInt, Jsonb};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Insertable)]
#[table_name="contracts"]
pub struct NewContract<'a> {
    pub address: &'a str,
    pub code_id: i64,
    pub creator: &'a str,
    pub admin: &'a str,
    pub label: &'a str,
    pub creation_time: &'a str,
    pub height: i64,
}

// TODO(gavin.doughtie): These are out of date and we're just
// using the diesel fields directly right now, but it's going
// to be better to move to these structs in the long run so
// leaving them here for reference.

#[derive(Queryable)]
pub struct Contract {
    pub address: Text,
    pub code_id: BigInt,
    pub creator: Text,
    pub admin: Text,
    pub label: Text,
    pub creation_time: Text,
    pub height: BigInt,
    pub json: Jsonb
}

#[derive(Insertable)]
#[table_name="cw20_balances"]
pub struct NewCw20Balance<'a> {
    pub address: &'a str,
    pub token: &'a str,
    pub balance: i64,
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
    pub decimals: i32
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cw20 {
    pub cw20_code_id: i64,
    pub label: String,
    pub msg: Cw20Msg,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GovToken {
    pub instantiate_new_cw20: Cw20
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewDao {
    pub description: String,
    pub gov_token: GovToken
}

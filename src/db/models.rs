use super::schema::{contracts, cw20_balances};
use diesel::sql_types::{Text, BigInt, Jsonb};

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

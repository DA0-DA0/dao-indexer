use super::schema::contracts;
use super::schema::block;
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
    pub height: i64
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
#[table_name="block"]
pub struct NewBlock<'a> {
    pub height: i64,
    pub hash: &'a str,
    pub num_txs: i64,
    // pub total_gas: i64,
    // pub proposer_address: &'a str,
}

#[derive(Queryable)]
pub struct Block {
    pub height: BigInt,
    pub hash: Text,
    pub num_txs: BigInt,
    // pub total_gas: BigInt,
    // pub proposer_address: Text
}

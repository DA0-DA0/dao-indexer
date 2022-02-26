use super::schema::contracts;
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

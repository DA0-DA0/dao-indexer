use super::contract_util::ContractAddresses;
use crate::db::models::{Dao, NewDao};
use bigdecimal::BigDecimal;
pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::gov_token::insert_gov_token;

use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;

pub fn insert_dao(
    db: &PgConnection,
    instantiate_dao: &Cw3DaoInstantiateMsg,
    contract_addr: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> anyhow::Result<()> {
    use crate::db::schema::dao::dsl::*;

    let dao_address = contract_addr.dao_address.as_ref().unwrap();

    let inserted_token_id: i32 =
        insert_gov_token(db, &instantiate_dao.gov_token, contract_addr, height).unwrap();

    let dao_model = NewDao::from_msg(
        dao_address,
        contract_addr.staking_contract_address.as_ref().unwrap(),
        inserted_token_id,
        instantiate_dao,
    );

    diesel::insert_into(dao)
        .values(dao_model)
        .execute(db as &PgConnection)
        .expect("Error saving dao");

    Ok(())
}

pub fn get_dao(db: &PgConnection, dao_address: &str) -> QueryResult<Dao> {
    use crate::db::schema::dao::dsl::*;
    dao.filter(contract_address.eq(dao_address))
        .first::<Dao>(db)
}

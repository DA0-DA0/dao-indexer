use super::contract_util::ContractAddresses;
use super::gov_token::insert_gov_token;
use crate::db::models::{Dao, NewDao};
use crate::indexing::indexer_registry::IndexerRegistry;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;

// use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use cw3_dao::msg::GovTokenMsg;

pub fn insert_dao(
    db: &IndexerRegistry,
    dao_name: &str,
    dao_description: &str,
    gov_token: &GovTokenMsg,
    dao_image_url: Option<&String>,
    contract_addr: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> anyhow::Result<()> {
    use crate::db::schema::dao::dsl::*;

    let dao_address = contract_addr
        .contract_address
        .as_ref()
        .ok_or_else(|| anyhow!("No contract address for DAO"))?;

    let mut gta_option = None;
    let gta: String;
    if let GovTokenMsg::UseExistingCw20 { addr, label: _ , ..} = gov_token {
        gta = addr.clone();
        gta_option = Some(&gta);
    } else if let Some(cw20_address) = &contract_addr.cw20_address {
        gta = cw20_address.clone();
        gta_option = Some(&gta);
    }

    let _ = insert_gov_token(db, gov_token, contract_addr, height)?;

    let dao_model = NewDao::new(
        dao_address,
        dao_description,
        gta_option,
        dao_image_url,
        dao_name,
        dao_address,
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

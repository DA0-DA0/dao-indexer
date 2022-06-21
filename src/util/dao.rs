use super::contract_util::ContractAddresses;
use super::gov_token::{insert_gov_token, insert_gov_token25};
use crate::db::models::{Dao, NewDao, NewMultisig};
use cw3_dao_2_5::msg::GovTokenMsg as GovTokenMsg25;

use crate::indexing::event_map::EventMap;
use crate::indexing::indexer_registry::IndexerRegistry;

use anyhow::anyhow;
use bigdecimal::BigDecimal;
pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::{error, warn};
use std::str::FromStr;

use cw3_dao::msg::GovTokenMsg;

pub fn get_single_event_item<'a>(events: &'a EventMap, key: &str, default: &'a str) -> &'a str {
    if let Some(values) = events.get(key) {
        if !values.is_empty() {
            return &values[0];
        }
    }
    default
}

pub fn get_tx_height_from_events(events: &EventMap) -> BigDecimal {
    let mut tx_height_opt = None;

    if let Some(tx_height_strings) = events.get("tx.height") {
        if !tx_height_strings.is_empty() {
            if tx_height_strings.len() > 1 {
                warn!("Expected one tx_height, but got: {:#?}", tx_height_strings);
            }
            let tx_height_str = &tx_height_strings[0];
            match BigDecimal::from_str(tx_height_str) {
                Ok(tx_height) => {
                    tx_height_opt = Some(tx_height);
                }
                Err(e) => {
                    error!("Error parsing tx_height string {} {:?}", tx_height_str, e);
                }
            }
        }
    }

    if let Some(height) = tx_height_opt {
        height
    } else {
        BigDecimal::default()
    }
}

pub fn insert_dao_25(
    db: &IndexerRegistry,
    dao_name: &str,
    dao_description: &str,
    gov_token: &GovTokenMsg25,
    dao_image_url: Option<&String>,
    contract_addr: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> anyhow::Result<()> {
    let mut gta_option = None;
    let gta: String;
    if let GovTokenMsg25::UseExistingCw20 { addr, label: _, .. } = gov_token {
        gta = addr.clone();
        gta_option = Some(&gta);
    } else if let Some(cw20_address) = contract_addr.cw20_address {
        gta = cw20_address.to_string();
        gta_option = Some(&gta);
    }
    let _ = insert_gov_token25(db, gov_token, contract_addr, height)?;
    insert_dao_private(
        db,
        dao_name,
        dao_description,
        gta_option,
        dao_image_url,
        contract_addr,
        height,
    )
}

pub fn insert_dao(
    db: &IndexerRegistry,
    dao_name: &str,
    dao_description: &str,
    gov_token: &GovTokenMsg,
    dao_image_url: Option<&String>,
    contract_addr: &ContractAddresses,
    height: Option<&BigDecimal>,
) -> anyhow::Result<()> {
    let mut gta_option = None;
    let gta: String;
    if let GovTokenMsg::UseExistingCw20 { addr, label: _, .. } = gov_token {
        gta = addr.clone();
        gta_option = Some(&gta);
    } else if let Some(cw20_address) = contract_addr.cw20_address {
        gta = cw20_address.to_string();
        gta_option = Some(&gta);
    }
    let _ = insert_gov_token(db, gov_token, contract_addr, height)?;
    insert_dao_private(
        db,
        dao_name,
        dao_description,
        gta_option,
        dao_image_url,
        contract_addr,
        height,
    )
}

fn insert_dao_private(
    db: &IndexerRegistry,
    dao_name: &str,
    dao_description: &str,
    gta_option: Option<&String>,
    dao_image_url: Option<&String>,
    contract_addr: &ContractAddresses,
    _height: Option<&BigDecimal>,
) -> anyhow::Result<()> {
    use crate::db::schema::dao::dsl::*;

    let dao_address = contract_addr
        .contract_address
        .as_ref()
        .ok_or_else(|| anyhow!("No contract address for DAO"))?;

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
        .on_conflict_do_nothing()
        .execute(db as &PgConnection)?;

    Ok(())
}

pub fn get_dao(db: &PgConnection, dao_address: &str) -> QueryResult<Dao> {
    use crate::db::schema::dao::dsl::*;
    dao.filter(contract_address.eq(dao_address))
        .first::<Dao>(db)
}

pub fn insert_multisig(
    db: &IndexerRegistry,
    dao_name: &str,
    dao_description: &str,
    dao_image_url: Option<&String>,
    contract_addr: &ContractAddresses,
) -> anyhow::Result<()> {
    use crate::db::schema::dao::dsl::*;

    let dao_address = contract_addr
        .contract_address
        .as_ref()
        .ok_or_else(|| anyhow!("No contract address for DAO"))?;

    let dao_model = NewMultisig::new(
        dao_address,
        dao_description,
        dao_image_url,
        dao_name,
        dao_address,
    );

    diesel::insert_into(dao)
        .values(dao_model)
        .on_conflict_do_nothing()
        .execute(db as &PgConnection)?;

    Ok(())
}

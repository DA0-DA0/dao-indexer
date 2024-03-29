use crate::db::models::NewContract;
use crate::indexing::event_map::EventMap;
use anyhow::anyhow;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::error;

#[derive(Debug)]
pub struct ContractAddresses<'a> {
    pub contract_address: Option<&'a str>,
    pub cw20_address: Option<&'a str>,
    pub staking_contract_address: Option<&'a str>,
}

pub fn get_contract_addresses<'a>(transaction_events: &'a EventMap) -> ContractAddresses<'a> {
    let mut contract_address: Option<&'a str> = None;
    let mut cw20_address: Option<&'a str> = None;
    let mut staking_contract_address: Option<&'a str> = None;

    if let Some(addr) = transaction_events.get("instantiate._contract_address") {
        // 0: DAO
        // 1: cw20
        // 2: staking contract
        // But if you use an existing token, you'll just get
        // DAO/staking contract
        if addr.len() == 3 {
            contract_address = Some(&addr[0]);
            cw20_address = Some(&addr[1]);
            staking_contract_address = Some(&addr[2]);
        } else if addr.len() == 2 {
            contract_address = Some(&addr[0]);
            staking_contract_address = Some(&addr[1]);
        } else if addr.len() == 1 {
            contract_address = Some(&addr[0]);
        } else {
            error!("unexpected addr {:?}", addr);
        }
    }
    ContractAddresses {
        contract_address,
        cw20_address,
        staking_contract_address,
    }
}

pub fn insert_contract(db: &PgConnection, contract_model: &NewContract) -> anyhow::Result<()> {
    use crate::db::schema::contracts::dsl::*;
    match diesel::insert_into(contracts)
        .values(contract_model)
        .execute(db)
    {
        Ok(_rows) => Ok(()),
        Err(e) => Err(anyhow!("Error: {:?}", e)),
    }
}

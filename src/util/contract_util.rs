use crate::db::models::NewContract;
use crate::indexing::event_map::EventMap;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::error;
use anyhow::anyhow;

#[derive(Debug)]
pub struct ContractAddresses {
    pub dao_address: Option<String>,
    pub cw20_address: Option<String>,
    pub staking_contract_address: Option<String>,
}

pub fn get_contract_addresses(transaction_events: &EventMap) -> ContractAddresses {
    let mut dao_address = None;
    let mut cw20_address = None;
    let mut staking_contract_address = None;

    if let Some(addr) = transaction_events.get("instantiate._contract_address") {
        // 0: DAO
        // 1: cw20
        // 2: staking contract
        // But if you use an existing token, you'll just get
        // DAO/staking contract
        if addr.len() == 3 {
            dao_address = Some(addr[0].clone());
            cw20_address = Some(addr[1].clone());
            staking_contract_address = Some(addr[2].clone());
        } else if addr.len() == 2 {
            dao_address = Some(addr[0].clone());
            staking_contract_address = Some(addr[1].clone());
        } else {
            error!("unexpected addr {:?}", addr);
        }
    }
    ContractAddresses {
        dao_address,
        cw20_address,
        staking_contract_address,
    }
}

pub fn insert_contract(
    db: &PgConnection,
    contract_model: &NewContract,
) -> anyhow::Result<()> {
    use crate::db::schema::contracts::dsl::*;
    match diesel::insert_into(contracts)
        .values(contract_model)
        .execute(db)
    {
        Ok(_rows) => Ok(()),
        Err(e) => Err(anyhow!("Error: {:?}", e)),
    }
}

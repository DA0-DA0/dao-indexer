use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};
use crate::util::dao::insert_dao;
use bigdecimal::BigDecimal;
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use std::collections::BTreeMap;
use std::str::FromStr;

impl IndexMessage for MsgInstantiateContract {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if events.is_none() {
            // TODO(gavindoughtie): Definitely NOT ok!
            return Ok(());
        }
        let db;
        match &registry.db {
            Some(registry_db) => {
                db = registry_db;
            }
            _ => return Ok(()),
        }
        println!("Indexing MsgInstantiateContract, events: {:?}", events);
        let contract_addresses = get_contract_addresses(events);
        let dao_address = contract_addresses
            .dao_address
            .as_ref()
            .ok_or("no dao_address")?;
        let staking_contract_address = contract_addresses
            .staking_contract_address
            .as_ref()
            .ok_or("no staking_contract_address")?;
        let mut tx_height_opt = None;
        if let Some(event_map) = events {
            let tx_height_strings = event_map.get("tx.height").ok_or("No tx.height supplied")?;
            if !tx_height_strings.is_empty() {
                let tx_height_str = &tx_height_strings[0];
                tx_height_opt = Some(BigDecimal::from_str(tx_height_str)?);
            }
        }
        let tx_height: BigDecimal;
        if let Some(height) = tx_height_opt {
            tx_height = height;
        } else {
            tx_height = BigDecimal::default();
        }

        let contract_model =
            NewContract::from_msg(dao_address, staking_contract_address, &tx_height, self);
        if let Err(e) = insert_contract(db, &contract_model) {
            eprintln!("Error inserting contract {:?}\n{:?}", &contract_model, e);
        }
        let msg_str = String::from_utf8(self.msg.clone())?;
        match serde_json::from_str::<Cw3DaoInstantiateMsg>(&msg_str) {
            Ok(instantiate_dao) => {
                insert_dao(db, &instantiate_dao, &contract_addresses, Some(&tx_height))
            }
            Err(e) => {
                eprintln!("Error parsing instantiate msg:\n{}\n{:?}", &msg_str, e);
                Ok(())
            }
        }
    }
}

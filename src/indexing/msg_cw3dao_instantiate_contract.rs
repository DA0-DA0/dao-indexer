use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};
use crate::util::dao::insert_dao;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::GovTokenMsg;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use log::{debug, error, info};
use std::str::FromStr;

impl IndexMessage for Cw3DaoInstantiateMsg {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let db;
        match &registry.db {
            Some(registry_db) => {
                db = registry_db;
            }
            _ => return Ok(()),
        }
        // debug!("Indexing MsgInstantiateContract, events: {:?}", events);
        let contract_addresses = get_contract_addresses(events);
        let dao_address = contract_addresses
            .dao_address
            .as_ref()
            .ok_or_else(|| anyhow!("no dao_address in {:?}\n{:?}", contract_addresses, events))?;
        let staking_contract_address = contract_addresses
            .staking_contract_address
            .as_ref()
            .ok_or_else(|| anyhow!("no staking_contract_address"))?;
        let mut tx_height_opt = None;

        let tx_height_strings = events
            .get("tx.height")
            .ok_or_else(|| anyhow!("No tx.height supplied"))?;
        if !tx_height_strings.is_empty() {
            let tx_height_str = &tx_height_strings[0];
            tx_height_opt = Some(BigDecimal::from_str(tx_height_str)?);
        }

        let tx_height: BigDecimal;
        if let Some(height) = tx_height_opt {
            tx_height = height;
        } else {
            tx_height = BigDecimal::default();
        }

        insert_dao(
            registry,
            &self.name,
            &self.description,
            &self.gov_token,
            self.image_url.as_ref(),
            &contract_addresses,
            Some(&tx_height),
        )
      }
}

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
        // let contract_addresses = get_contract_addresses(events);
        // let dao_address = contract_addresses
        //     .dao_address
        //     .as_ref()
        //     .ok_or_else(|| anyhow!("no dao_address in {:?}\n{:?}", contract_addresses, events))?;
        // let staking_contract_address = contract_addresses
        //     .staking_contract_address
        //     .as_ref()
        //     .ok_or_else(|| anyhow!("no staking_contract_address"))?;
        // let mut tx_height_opt = None;

        // let tx_height_strings = events
        //     .get("tx.height")
        //     .ok_or_else(|| anyhow!("No tx.height supplied"))?;
        // if !tx_height_strings.is_empty() {
        //     let tx_height_str = &tx_height_strings[0];
        //     tx_height_opt = Some(BigDecimal::from_str(tx_height_str)?);
        // }

        // let tx_height: BigDecimal;
        // if let Some(height) = tx_height_opt {
        //     tx_height = height;
        // } else {
        //     tx_height = BigDecimal::default();
        // }

        // let contract_model =
        //     NewContract::from_msg(dao_address, staking_contract_address, &tx_height, self);
        // if let Err(e) = insert_contract(db, &contract_model) {
        //     error!("Error inserting contract {:?}\n{:?}", &contract_model, e);
        // }
        let msg_str = String::from_utf8(self.msg.clone())?;
        let parsed = serde_json::from_str::<serde_json::Value>(&msg_str)?;
        // registry.index_message_and_events(events, &parsed, &msg_str)?;

        match serde_json::from_str::<Cw3DaoInstantiateMsg>(&msg_str) {
            Ok(instantiate_dao) => insert_dao(
                registry,
                &instantiate_dao.name,
                &instantiate_dao.description,
                Some(instantiate_dao.gov_token),
                instantiate_dao.image_url.as_ref(),
                &contract_addresses,
                Some(&tx_height),
            ),
            Err(e) => {
                error!("Error parsing instantiate msg ({:?}); trying generic", e);
                info!("parsed:\n{}", serde_json::to_string_pretty(&parsed)?);
                let mut gov_token = None;
                if let Some(gov_token_dict) = parsed.get("gov_token") {
                    match serde_json::from_str::<GovTokenMsg>(&gov_token_dict.to_string()) {
                        Ok(msg) => {
                            gov_token = Some(msg);
                        }
                        _ => {
                            gov_token = None;
                        }
                    }
                }
                insert_dao(
                    registry,
                    &parsed["name"].to_string(),
                    &parsed["description"].to_string(),
                    gov_token,
                    Some(&parsed["image_url"].to_string()),
                    &contract_addresses,
                    Some(&tx_height),
                )
            }
        }
    }
}

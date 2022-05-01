use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};
use crate::util::dao::insert_dao;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use log::{debug, error, info};
use std::str::FromStr;
use cw3_dao::msg::GovTokenMsg;

impl IndexMessage for MsgInstantiateContract {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let db;
        match &registry.db {
            Some(registry_db) => {
                db = registry_db;
            }
            _ => return Ok(()),
        }
        debug!("Indexing MsgInstantiateContract, events: {:?}", events);
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

        let contract_model =
            NewContract::from_msg(dao_address, staking_contract_address, &tx_height, self);
        if let Err(e) = insert_contract(db, &contract_model) {
            error!("Error inserting contract {:?}\n{:?}", &contract_model, e);
        }
        let msg_str = String::from_utf8(self.msg.clone())?;

        // TODO(gavin.doughtie):
        // Due to versioning, we can't guarantee that serde deserialization
        // will work here so we have to deal with that OR import all the
        // different contract versions and try them in a cascade.
        // TODO(gavin.doughtie): This might be a lp contract:
        // {
        //   "lp_token_code_id": 1,
        //   "token1_denom": {
        //       "cw20": "juno17c7zyezg3m8p2tf9hqgue9jhahvle70d59e8j9nmrvhw9anrpk8qxlrghx"
        //   },
        //   "token2_denom": {
        //       "native": "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9"
        //   }
        // }
        match serde_json::from_str::<Cw3DaoInstantiateMsg>(&msg_str) {
            Ok(instantiate_dao) => insert_dao(
                registry,
                &instantiate_dao.name,
                &instantiate_dao.description,
                &instantiate_dao.gov_token,
                instantiate_dao.image_url.as_ref(),
                &contract_addresses,
                Some(&tx_height),
            ),
            Err(e) => {
                error!("Error parsing instantiate msg ({:?}); trying generic", e);
                let parsed = serde_json::from_str::<serde_json::Value>(&msg_str)?;
                info!("parsed:\n{}", serde_json::to_string_pretty(&parsed)?);
                let gov_token = serde_json::from_str::<GovTokenMsg>(&parsed["gov_token"].to_string())?;
                insert_dao(
                    registry,
                    &parsed["name"].to_string(),
                    &parsed["description"].to_string(),
                    &gov_token,
                    Some(&parsed["image_url"].to_string()),
                    &contract_addresses,
                    Some(&tx_height),
                )
            }
        }
    }
}

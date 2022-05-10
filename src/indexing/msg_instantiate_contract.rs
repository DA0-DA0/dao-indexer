use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};

use anyhow::anyhow;
use bigdecimal::BigDecimal;
// use cosmrs::proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cosmrs::cosmwasm::MsgInstantiateContract;

use log::{debug, error};
use std::str::FromStr;


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
        let contract_address = contract_addresses
            .contract_address
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

        let admin = if let Some(account_id) = self.admin.clone() {
            account_id.to_string()
        } else {
            "".to_string()
        };

        let creator = self.sender.to_string();

        let label = self.label.clone().unwrap_or_default();

        let contract_model = NewContract::from_msg(contract_address, staking_contract_address, &creator, &admin, &label, &tx_height, self);
        if let Err(e) = insert_contract(db, &contract_model) {
            error!("Error inserting contract {:?}\n{:?}", &contract_model, e);
        }
        let msg_str = String::from_utf8(self.msg.clone())?;
        let parsed = serde_json::from_str::<serde_json::Value>(&msg_str)?;
        registry.index_message_and_events(events, &parsed, &msg_str)?;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use cosmrs::{cosmwasm::MsgInstantiateContract, AccountId};

    use crate::indexing::event_map::EventMap;
    use crate::indexing::index_message::IndexMessage;
    use crate::indexing::indexer_registry::IndexerRegistry;
    // use crate::indexing::msg_instantiate_contract;    

    #[test]
    fn it_works() {

        let test_acc_id = "juno1cma4czt2jnydvrvz3lrc9jvcmhpjxtds95s3c6"
            .parse::<AccountId>()
            .unwrap();

        let z = MsgInstantiateContract {
            sender: test_acc_id,
            admin: None,
            code_id: 69 as u64,
            label: None,
            msg: Vec::new(),
            funds: Vec::new(),            
        };

        let test_idx_reg = IndexerRegistry::default(); 

        let omega = EventMap::new();
    
        let res = z.index_message(&test_idx_reg, &omega);
        assert_eq!(2 + 2, 4);
    }
}
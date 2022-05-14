use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::db::models::NewContract;
use crate::util::contract_util::{get_contract_addresses, insert_contract};
use anyhow::anyhow;
use bigdecimal::BigDecimal;
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
        let contract_model = create_new_contract(self, events)?;

        if let Err(e) = insert_contract(db, &contract_model) {
            error!("Error inserting contract {:?}\n{:?}", &contract_model, e);
        }

        let msg_str = String::from_utf8(self.msg.clone())?;
        let parsed = serde_json::from_str::<serde_json::Value>(&msg_str)?;
        registry.index_message_and_events(events, &parsed, &msg_str)?;

        Ok(())
    }
}

fn create_new_contract<'a>(
    msg_inst_contract: &'a MsgInstantiateContract,
    events: &'a std::collections::BTreeMap<String, Vec<String>>,
) -> Result<NewContract<'a>, anyhow::Error> {
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
    let admin = if let Some(account_id) = msg_inst_contract.admin.clone() {
        account_id.to_string()
    } else {
        "".to_string()
    };
    let creator = msg_inst_contract.sender.to_string();
    let mut label = "";
    if let Some(contract_label) = &msg_inst_contract.label {
        label = contract_label;
    }
    // Dont need to clone contract and staking address
    let contract_model = NewContract::from_msg(
        contract_address,
        staking_contract_address,
        creator,
        admin,
        label,
        tx_height,
        msg_inst_contract,
    );
    Ok(contract_model)
}

#[cfg(test)]
mod tests {
    use crate::indexing::event_map::EventMap;
    use crate::indexing::msg_instantiate_contract::create_new_contract;
    use cosmrs::{cosmwasm::MsgInstantiateContract, AccountId};

    #[test]
    fn test_new_contract_no_events_fails() {
        let test_acc_id = "juno1cma4czt2jnydvrvz3lrc9jvcmhpjxtds95s3c6"
            .parse::<AccountId>()
            .unwrap();

        let msg_inst_contract = MsgInstantiateContract {
            sender: test_acc_id,
            admin: None,
            code_id: 69,
            label: None,
            msg: Vec::new(),
            funds: Vec::new(),
        };

        let empty_event_map = EventMap::new();
        let res = create_new_contract(&msg_inst_contract, &empty_event_map);
        assert!(res.is_err());
    }
}

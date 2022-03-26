use super::index::Index;
use super::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
use crate::util::update_balance::update_balance;
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use std::collections::BTreeMap;
use std::str::FromStr;

impl Index for Cw20ExecuteMsg {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        dump_events(events);
        if let Some(db) = &registry.db {
            if let Some(event_map) = events {
                if let Some(wasm_actions) = event_map.get("wasm.action") {
                    if !wasm_actions.is_empty() && &wasm_actions[0] == "send" {
                        let tx_height =
                            BigDecimal::from_str(&(event_map.get("tx.height").unwrap()[0]))?;
                        let contract_addresses = event_map
                            .get("wasm._contract_address")
                            .ok_or("no wasm._contract_address")?;
                        let gov_token_address = &contract_addresses[0];
                        let to_addresses = event_map.get("wasm.to").ok_or("no wasm.to")?;
                        let staking_contract_addr = to_addresses[0].clone();
                        let amounts = &event_map.get("wasm.amount").ok_or("no wasm.amount")?;
                        let senders = event_map.get("wasm.from").ok_or("no wasm.from")?;
                        let sender_addr = &senders[0];
                        let mut send_amount: &str = &amounts[0];

                        let receiving_contract_action: &str;
                        if wasm_actions.len() > 1 {
                            receiving_contract_action = &wasm_actions[1];
                        } else {
                            receiving_contract_action = "";
                        }
                        let action_amount: &str = &amounts[1];
                        if receiving_contract_action == "stake" {
                            send_amount = action_amount;
                        }
                        let balance_update: Cw20Coin = Cw20Coin {
                            address: staking_contract_addr,
                            amount: Uint128::from_str(send_amount)?,
                        };
                        update_balance(
                            db,
                            Some(&tx_height),
                            gov_token_address,
                            sender_addr,
                            &balance_update,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

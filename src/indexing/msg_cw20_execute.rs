use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
use crate::util::update_balance::update_balance;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use std::str::FromStr;
use log::error;

impl IndexMessage for Cw20ExecuteMsg {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        event_map: &EventMap,
    ) -> anyhow::Result<()> {
        dump_events(event_map);
        if let Some(wasm_actions) = event_map.get("wasm.action") {
            if !wasm_actions.is_empty() && &wasm_actions[0] == "send" {
                let tx_height = BigDecimal::from_str(&(event_map.get("tx.height").unwrap()[0]))?;
                let contract_addresses = event_map
                    .get("wasm._contract_address")
                    .ok_or_else(|| anyhow!("no wasm._contract_address"))?;
                let gov_token_address = &contract_addresses[0];
                let to_addresses = event_map
                    .get("wasm.to")
                    .ok_or_else(|| anyhow!("no wasm.to"))?;
                let staking_contract_addr = to_addresses[0].clone();
                let amounts = &event_map
                    .get("wasm.amount")
                    .ok_or_else(|| anyhow!("no wasm.amount"))?;
                let senders = event_map
                    .get("wasm.from")
                    .ok_or_else(|| anyhow!("no wasm.from"))?;
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
                let mut amount: Uint128 = Uint128::new(0);
                match Uint128::from_str(send_amount) {
                    Ok(parsed_amount) => {
                        amount = parsed_amount;
                    }
                    Err(e) => {
                        // Try to parse as a decimal
                        let decimal_amount = BigDecimal::from_str(send_amount)?;
                        error!("Parsed as {:?} due to error {:?}", decimal_amount, e)
                    }
                }
                let balance_update: Cw20Coin = Cw20Coin {
                    address: staking_contract_addr,
                    amount,
                };
                update_balance(
                    registry,
                    Some(&tx_height),
                    gov_token_address,
                    sender_addr,
                    &balance_update,
                )?;
            }
        }
        Ok(())
    }
}

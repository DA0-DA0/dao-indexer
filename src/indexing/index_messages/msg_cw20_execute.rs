use crate::indexing::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::util::debug::{dump_events, events_string};
use crate::util::update_balance::update_balance;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
use cosmwasm_std::Uint128;
pub use cw20::Cw20ExecuteMsg;
use log::error;
use std::str::FromStr;

impl IndexMessage for Cw20ExecuteMsg {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        event_map: &EventMap,
    ) -> anyhow::Result<()> {
        dump_events(event_map);
        if let Some(wasm_actions) = event_map.get("wasm.action") {
            if !wasm_actions.is_empty() && &wasm_actions[0] == "send" {
                let tx_height = BigDecimal::from_str(
                    &({
                        let this = event_map.get("tx.height");
                        if let Some(val) = this {
                            val
                        } else {
                            error!("{}", events_string(event_map));
                            panic!("called `Option::unwrap()` on a `None` value")
                        }
                    }[0]),
                )?;
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

                let receiving_contract_action: &str = if wasm_actions.len() > 1 {
                    &wasm_actions[1]
                } else {
                    ""
                };

                let action_amount: &str = if amounts.len() > 1 {
                    &amounts[1]
                } else {
                    error!("Expected more than one amount, but got: {:?}", amounts);
                    "0"
                };

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
                if registry.db.is_some() {
                    update_balance(
                        registry,
                        Some(&tx_height),
                        gov_token_address,
                        sender_addr,
                        &staking_contract_addr,
                        u128::from(amount),
                    )?;
                }
            }
        }
        Ok(())
    }
}

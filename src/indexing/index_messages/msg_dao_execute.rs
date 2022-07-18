use crate::indexing::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::util::debug::{dump_events, dump_execute_contract};
use crate::util::update_balance::update_balance_from_events;
use anyhow::anyhow;
use cw3_dao::msg::ExecuteMsg;

impl IndexMessage for ExecuteMsg {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        event_map: &EventMap,
    ) -> anyhow::Result<()> {
        if registry.db.is_none() {
            return Err(anyhow!("No db connection available"));
        }
        dump_execute_contract(self);
        dump_events(event_map);

        if let Some(wasm_actions) = event_map.get("wasm.action") {
            // TODO(gavin.doughtie): Handle propose, vote
            if !wasm_actions.is_empty() && wasm_actions[0] == "execute" {
                for (i, action_type) in (&wasm_actions[1..]).iter().enumerate() {
                    match action_type.as_str() {
                        "transfer" => {
                            if let Err(e) = update_balance_from_events(registry, i, event_map) {
                                return Err(anyhow!(e));
                            }
                        }
                        "mint" => {
                            if let Err(e) = update_balance_from_events(registry, i, event_map) {
                                return Err(anyhow!(e));
                            }
                        }
                        _ => {
                            return Err(anyhow!("Unhandled exec type {}", action_type));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

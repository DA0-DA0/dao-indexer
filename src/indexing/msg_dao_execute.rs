use super::index::Index;
use crate::util::debug::{dump_events, dump_execute_contract};
use crate::util::update_balance::update_balance_from_events;
use cw3_dao::msg::ExecuteMsg;
use diesel::pg::PgConnection;
use std::collections::BTreeMap;

impl Index for ExecuteMsg {
  fn index(
    &self,
    db: &PgConnection,
    events: &Option<BTreeMap<String, Vec<String>>>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    dump_execute_contract(self);
    dump_events(events);
    if let Some(event_map) = events {
      if let Some(wasm_actions) = event_map.get("wasm.action") {
        // TODO(gavin.doughtie): Handle propose, vote
        if !wasm_actions.is_empty() && wasm_actions[0] == "execute" {
          for (i, action_type) in (&wasm_actions[1..]).iter().enumerate() {
            match action_type.as_str() {
              "transfer" => {
                if let Err(e) = update_balance_from_events(db, i, event_map) {
                  return Err(Box::from(e));
                }
              }
              "mint" => {
                if let Err(e) = update_balance_from_events(db, i, event_map) {
                  return Err(Box::from(e));
                }
              }
              _ => {
                return Err(Box::from(format!("Unhandled exec type {}", action_type)));
              }
            }
          }
        }
      }
    }
    Ok(())
  }
}

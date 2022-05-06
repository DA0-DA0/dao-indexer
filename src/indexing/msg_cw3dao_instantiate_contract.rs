use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::util::contract_util::get_contract_addresses;
use crate::util::dao::insert_dao;
use anyhow::anyhow;
use bigdecimal::BigDecimal;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use std::str::FromStr;

impl IndexMessage for Cw3DaoInstantiateMsg {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let contract_addresses = get_contract_addresses(events);
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

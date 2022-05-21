use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::util::contract_util::get_contract_addresses;
use crate::util::dao::{get_tx_height_from_events, insert_dao, insert_dao_25};

use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;

impl IndexMessage for Cw3DaoInstantiateMsg {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let contract_addresses = get_contract_addresses(events);
        let tx_height = get_tx_height_from_events(events);
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

impl IndexMessage for Cw3DaoInstantiateMsg25 {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let contract_addresses = get_contract_addresses(events);
        let tx_height = get_tx_height_from_events(events);
        insert_dao_25(
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

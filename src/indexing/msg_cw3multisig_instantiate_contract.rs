use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::util::contract_util::get_contract_addresses;
use crate::util::dao::insert_multisig;
use cw3_multisig::msg::InstantiateMsg as Cw3MultisigInstantiateMsg;

impl IndexMessage for Cw3MultisigInstantiateMsg {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()> {
        let contract_addresses = get_contract_addresses(events);
        insert_multisig(
            registry,
            &self.name,
            &self.description,
            self.image_url.as_ref(),
            &contract_addresses,
        )
    }
}

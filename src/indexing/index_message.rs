use super::event_map::EventMap;
use super::indexer_registry::IndexerRegistry;

pub trait IndexMessage {
    fn index_message(&self, registry: &IndexerRegistry, events: &EventMap) -> anyhow::Result<()>;
}

impl IndexMessage for () {
    fn index_message(&self, _registry: &IndexerRegistry, _events: &EventMap) -> anyhow::Result<()> {
        Ok(())
    }
}

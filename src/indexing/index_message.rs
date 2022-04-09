use super::event_map::EventMap;
use super::indexer_registry::IndexerRegistry;

pub trait IndexMessage {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        events: &EventMap,
    ) -> anyhow::Result<()>;
}

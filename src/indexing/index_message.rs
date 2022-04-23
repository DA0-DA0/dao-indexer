use super::event_map::EventMap;
use super::indexer_registry::IndexerRegistry;
use diesel::PgConnection;

pub trait IndexMessage {
    fn index_message(
        &self,
        conn: Option<&PgConnection>,
        registry: &IndexerRegistry,
        events: &EventMap,
    ) -> anyhow::Result<()>;
}

impl IndexMessage for () {
    fn index_message(
        &self,
        _conn: Option<&PgConnection>,
        _registry: &IndexerRegistry,
        _events: &EventMap,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

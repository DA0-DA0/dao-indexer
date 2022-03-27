use std::collections::BTreeMap;

use super::indexer_registry::IndexerRegistry;

pub trait IndexMessage {
    fn index_message(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>>; // TODO(gavindoughtie): anyhow::Result<()>
}

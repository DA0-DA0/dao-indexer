use std::collections::BTreeMap;

use super::indexer_registry::IndexerRegistry;

pub trait Index {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>>; // TODO(gavindoughtie): anyhow::Result<()>
}

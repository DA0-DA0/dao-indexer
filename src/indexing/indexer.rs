use serde_json::Value;
use std::collections::BTreeMap;
use std::slice::Iter;

use super::indexer_registry::IndexerRegistry;

pub trait Indexer {
    fn index(
        &self,
        registry: &IndexerRegistry,
        events: &Option<BTreeMap<String, Vec<String>>>,
        msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn id(&self) -> String;
    fn registry_keys(&self) -> Iter<String>;
    fn extract_message_key(&self, msg: &Value, msg_string: &str) -> Option<String>;
}

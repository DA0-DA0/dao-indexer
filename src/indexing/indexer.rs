use super::event_map::EventMap;
use serde_json::Value;
use std::slice::Iter;

use super::indexer_registry::{IndexerRegistry, RegistryKey};

pub trait Indexer {
    // Indexes a message and its transaction events
    fn index(
        &self,
        // The registry of indexers
        registry: &IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        events: &EventMap,
        // Generic serde-parsed value dictionary
        msg_dictionary: &Value,
        // The decoded string value of the message
        msg_str: &str,
    ) -> anyhow::Result<()>;

    // ID of this indexer. Used internally in indexer implementations
    // and in debugging.
    fn id(&self) -> String;

    // Keys that this indexer wants to have its "index" method called for.
    fn registry_keys(&self) -> Iter<RegistryKey>;

    // Extract the key from a given message. This should be one of the keys
    // returened in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey>;
}

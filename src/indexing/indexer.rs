use super::event_map::EventMap;
use serde_json::Value;
use std::slice::Iter;

use super::indexer_registry::{IndexerRegistry, RegistryKey};

pub trait Indexer<'a> {
    type MessageType: serde::Deserialize<'a>;
    // Indexes a message and its transaction events
    fn index_impl(
        &self,
        // The registry of indexers
        registry: &IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        events: &EventMap,
        // Generic serde-parsed value dictionary
        msg_dictionary: &Value,
        // The decoded string value of the message
        msg_str: &str,
    ) -> anyhow::Result<()> {
        let execute_contract = serde_json::from_str::<Self::MessageType>(msg_str)?;
        execute_contract.index_message(registry, events)
    }

    // ID of this indexer. Used internally in indexer implementations
    // and in debugging.
    fn id(&self) -> String;

    // Keys that this indexer wants to have its "index" method called for.
    fn registry_keys(&self) -> Iter<RegistryKey>;

    // Extract the key from a given message. This should be one of the keys
    // returened in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey>;
}

pub trait IndexerWrapper {
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
}

impl<'a, I: Indexer<'a>> IndexerWrapper for I {
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
    ) -> anyhow::Result<()> {
        self.index_impl(registry, events, msg_dictionary, msg_str)
    }
}

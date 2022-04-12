use super::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::slice::Iter;

use super::indexer_registry::{IndexerRegistry, RegistryKey};

pub trait Indexer {
    type MessageType: DeserializeOwned + IndexMessage;
    // Indexes a message and its transaction events
    fn index<'a>(
        &'a self,
        // The registry of indexers
        registry: &'a IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        events: &'a EventMap,
        // Generic serde-parsed value dictionary
        _msg_dictionary: &'a Value,
        // The decoded string value of the message
        msg_str: &'a str,
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

// IndexerDyn is needed in order to have dynamic dispatch on Indexer. Rust doesn't allow dynamic
// dispatch on traits with associated types.
// See https://users.rust-lang.org/t/dynamic-dispatch-and-associated-types/39584/2
pub trait IndexerDyn {
    fn index_dyn<'a>(
        &'a self,
        registry: &'a IndexerRegistry,
        events: &'a EventMap,
        msg_dictionary: &'a Value,
        msg_str: &'a str,
    ) -> anyhow::Result<()>;
    fn extract_message_key_dyn(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey>;
    fn registry_keys_dyn(&self) -> Iter<RegistryKey>;
    fn id(&self) -> String;
}

impl<I: Indexer> IndexerDyn for I {
    fn index_dyn<'a>(
        &'a self,
        registry: &'a IndexerRegistry,
        events: &'a EventMap,
        msg_dictionary: &'a Value,
        msg_str: &'a str,
    ) -> anyhow::Result<()> {
        self.index(registry, events, msg_dictionary, msg_str)
    }

    fn extract_message_key_dyn(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey> {
        self.extract_message_key(msg, msg_string)
    }

    fn registry_keys_dyn(&self) -> Iter<RegistryKey> {
        self.registry_keys()
    }

    fn id(&self) -> String {
        self.id()
    }
}

use super::event_map::EventMap;
use crate::indexing::index_message::IndexMessage;
use diesel::PgConnection;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::indexer_registry::{IndexerRegistry, RegistryKey};
pub type RootKeysType<'a> = Box<dyn Iterator<Item = &'a str> + 'a>;
pub type RegistryKeysType<'a> = Box<dyn Iterator<Item = &'a RegistryKey> + 'a>;

pub fn root_keys_from_iter<'a>(iter: impl Iterator<Item = &'a str> + 'a) -> RootKeysType<'a> {
    Box::new(iter)
}

pub fn registry_keys_from_iter<'a>(
    iter: impl Iterator<Item = &'a RegistryKey> + 'a,
) -> RegistryKeysType<'a> {
    Box::new(iter)
}

pub trait Indexer {
    type MessageType: DeserializeOwned + IndexMessage;

    // Indexes a message and its transaction events
    fn index<'a>(
        &'a self,
        conn: Option<&'a PgConnection>,
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
        execute_contract.index_message(conn, registry, events)
    }

    // ID of this indexer. Used internally in indexer implementations
    // and in debugging.
    fn id(&self) -> String;

    // Keys that this indexer wants to have its "index" method called for.
    fn registry_keys(&self) -> RegistryKeysType; // Box<dyn Iterator<Item = &RegistryKey> + '_>;

    // Iterator over the root keys in a given
    // message, used by the default extract_message_key
    // implementation
    fn root_keys(&self) -> RootKeysType; //Box<dyn Iterator<Item = &'a str> + 'a>;

    // Extract the key from a given message. This should be one of the keys
    // returned in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        let roots = self.root_keys();
        for key in roots {
            if msg.get(key).is_some() {
                return Some(RegistryKey::new(self.id()));
            }
        }
        None
    }
}

// IndexerDyn is needed in order to have dynamic dispatch on Indexer. Rust doesn't allow dynamic
// dispatch on traits with associated types.
// See https://users.rust-lang.org/t/dynamic-dispatch-and-associated-types/39584/2
pub trait IndexerDyn {
    fn index_dyn<'a>(
        &'a self,
        conn: Option<&'a PgConnection>,
        registry: &'a IndexerRegistry,
        events: &'a EventMap,
        msg_dictionary: &'a Value,
        msg_str: &'a str,
    ) -> anyhow::Result<()>;
    fn extract_message_key_dyn(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey>;
    fn registry_keys_dyn(&self) -> RegistryKeysType;
    fn id(&self) -> String;
}

impl<I: Indexer + Clone> IndexerDyn for I {
    fn index_dyn<'a>(
        &'a self,
        conn: Option<&'a PgConnection>,
        registry: &'a IndexerRegistry,
        events: &'a EventMap,
        msg_dictionary: &'a Value,
        msg_str: &'a str,
    ) -> anyhow::Result<()> {
        self.index(conn, registry, events, msg_dictionary, msg_str)
    }

    fn extract_message_key_dyn(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey> {
        self.extract_message_key(msg, msg_string)
    }

    fn registry_keys_dyn(&self) -> RegistryKeysType {
        self.registry_keys()
    }

    fn id(&self) -> String {
        self.id()
    }
}

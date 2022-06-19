use super::event_map::EventMap;
use super::indexer_registry::{IndexerRegistry, RegistryKey};
use crate::db::db_builder::DatabaseBuilder;
use crate::indexing::index_message::IndexMessage;
use log::{error, warn};
use serde::de::DeserializeOwned;
use serde_json::Value;
pub type RootKeyType<'a> = dyn Iterator<Item = &'a String> + 'a;
pub type RootKeysType<'a> = Box<RootKeyType<'a>>;
pub type RegistryKeysType<'a> = Box<dyn Iterator<Item = &'a RegistryKey> + 'a>;

pub fn root_keys_from_iter<'a>(iter: impl Iterator<Item = &'a String> + 'a) -> RootKeysType<'a> {
    Box::new(iter)
}

pub fn registry_keys_from_iter<'a>(
    iter: impl Iterator<Item = &'a RegistryKey> + 'a,
) -> RegistryKeysType<'a> {
    Box::new(iter)
}

fn has_all(keys: RootKeysType, msg: &Value) -> bool {
    for key in keys {
        if msg.get(key).is_none() {
            return false;
        }
    }
    true
}

pub trait Indexer {
    type MessageType: DeserializeOwned + IndexMessage;

    /// Called once at startup; indexers can create DB tables,
    /// initialize static lookups, etc.
    /// No calls to index will be made until all indexers
    /// have initialized.
    fn initialize<'a>(&'a self, _registry: &'a IndexerRegistry) -> anyhow::Result<()> {
        println!("initialize called on {}", self.id());
        Ok(())
    }

    fn initialize_schemas(&mut self, _builder: &mut DatabaseBuilder) -> anyhow::Result<()> {
        // Implementors can do whatevah
        Ok(())
    }

    // Indexes a message and its transaction events
    fn index<'a>(
        &'a self,
        // The registry of indexers
        registry: &'a IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        events: &'a EventMap,
        // Generic serde-parsed value dictionary
        msg_dictionary: &'a Value,
        // The decoded string value of the message
        msg_str: &'a str,
    ) -> anyhow::Result<()> {
        match serde_json::from_str::<Self::MessageType>(msg_str) {
            Ok(msg) => msg.index_message(registry, events),
            Err(e) => {
                error!("{} Error deserializing {:#?}", self.id(), e);
                self.index_message_dictionary(registry, events, msg_dictionary, msg_str)
            }
        }
    }

    fn index_message_dictionary<'a>(
        &'a self,
        // The registry of indexers
        _registry: &'a IndexerRegistry,
        // All the transaction events in a map of "event.id": Vec<String> values.
        _events: &'a EventMap,
        // Generic serde-parsed value dictionary
        msg_dictionary: &'a Value,
        // The decoded string value of the message
        _msg_str: &'a str,
    ) -> anyhow::Result<()> {
        warn!(
            "{} failed to deserialize and no message dictionary handler for message\n{:#?}",
            self.id(),
            msg_dictionary
        );
        Ok(())
    }

    // ID of this indexer. Used internally in indexer implementations
    // and in debugging.
    fn id(&self) -> String;

    // Keys that this indexer wants to have its "index" method called for.
    fn registry_keys(&self) -> RegistryKeysType;

    // Iterator over the root keys in a given
    // message, used by the default extract_message_key
    // implementation
    fn root_keys(&self) -> RootKeysType;

    // Iterator over the root keys in a given
    // message, used by the default extract_message_key
    // implementation. If a message contains ALL of these
    // keys, then it is definitely of the required type.
    fn required_root_keys(&self) -> RootKeysType;

    fn has_required_root_keys(&self) -> bool {
        false
    }

    fn first_matching_key(&self, msg: &Value) -> Option<RegistryKey> {
        let roots = self.root_keys();
        for key in roots {
            if msg.get(key).is_some() {
                return Some(RegistryKey::new(self.id()));
            }
        }
        None
    }

    // Extract the key from a given message. This should be one of the keys
    // returned in registry_keys or None.
    fn extract_message_key(&self, msg: &Value, _msg_string: &str) -> Option<RegistryKey> {
        if self.has_required_root_keys() {
            let required_roots = self.required_root_keys();
            if has_all(required_roots, msg) {
                return Some(RegistryKey::new(self.id()));
            }
            return None;
        }
        self.first_matching_key(msg)
    }
}

// IndexerDyn is needed in order to have dynamic dispatch on Indexer. Rust doesn't allow dynamic
// dispatch on traits with associated types.
// See https://users.rust-lang.org/t/dynamic-dispatch-and-associated-types/39584/2
pub trait IndexerDyn {
    fn initialize_dyn<'a>(&'a self, registry: &'a IndexerRegistry) -> anyhow::Result<()>;
    fn index_dyn<'a>(
        &'a self,
        registry: &'a IndexerRegistry,
        events: &'a EventMap,
        msg_dictionary: &'a Value,
        msg_str: &'a str,
    ) -> anyhow::Result<()>;
    fn initialize_schemas_dyn<'a>(&'a mut self, builder: &'a mut DatabaseBuilder) -> anyhow::Result<()>;
    fn extract_message_key_dyn(&self, msg: &Value, msg_string: &str) -> Option<RegistryKey>;
    fn registry_keys_dyn(&self) -> RegistryKeysType;
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

    fn initialize_dyn<'a>(&'a self, registry: &'a IndexerRegistry) -> anyhow::Result<()> {
        self.initialize(registry)
    }

    fn initialize_schemas_dyn<'a>(&'a mut self, builder: &'a mut DatabaseBuilder) -> anyhow::Result<()> {
        self.initialize_schemas(builder)
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

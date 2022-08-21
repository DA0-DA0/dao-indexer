use super::event_map::EventMap;
use super::indexer::{Indexer, IndexerDyn};
use crate::db::db_builder::DatabaseBuilder;
use crate::db::persister::{Persister, StubPersister};
use diesel::pg::PgConnection;
use log::{debug, error};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegistryKey(String);

impl RegistryKey {
    pub fn new(key: String) -> Self {
        RegistryKey(key)
    }
}

impl fmt::Display for RegistryKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for RegistryKey {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

pub trait Register {
    fn register(&mut self, indexer: Box<dyn IndexerDyn>, registry_key: Option<&str>);
}

pub struct IndexerRegistry {
    pub db: Option<PgConnection>,
    pub seaql_db: Option<DatabaseConnection>,
    pub db_builder: DatabaseBuilder,
    pub persister: Box<dyn Persister<Id = u64>>,
    /// Maps string key values to ids of indexers
    handlers: HashMap<RegistryKey, Vec<usize>>,
    indexers: Vec<Box<dyn IndexerDyn>>,
}

impl<'a> From<&'a IndexerRegistry> for &'a PgConnection {
    fn from(registry: &'a IndexerRegistry) -> Self {
        registry.db.as_ref().unwrap()
    }
}

impl Deref for IndexerRegistry {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        self.db.as_ref().unwrap()
    }
}

impl Default for IndexerRegistry {
    fn default() -> Self {
        IndexerRegistry::new(None, None, Box::from(StubPersister {}))
    }
}

impl<'a> IndexerRegistry {
    pub fn new(
        db: Option<PgConnection>,
        seaql_db: Option<DatabaseConnection>,
        persister: Box<dyn Persister<Id = u64>>,
    ) -> Self {
        IndexerRegistry {
            db,
            seaql_db,
            db_builder: DatabaseBuilder::default(),
            handlers: HashMap::default(),
            indexers: vec![],
            persister,
        }
    }

    pub fn initialize(&mut self) -> anyhow::Result<()> {
        for indexer in self.indexers.iter() {
            indexer.initialize_dyn(self)?;
        }
        for indexer in self.indexers.iter_mut() {
            indexer.initialize_schemas_dyn(&mut self.db_builder)?;
        }
        self.db_builder.finalize_columns();
        Ok(())
    }

    // This method gets handed the decoded cosmwasm message
    // and asks its registered indexers to index it if they can.
    pub fn index_message_and_events(
        &self,
        events: &EventMap,
        msg_dictionary: &Value,
        msg_str: &str,
    ) -> anyhow::Result<()> {
        if let Some(message_keys) = &self.extract_message_keys(msg_dictionary, msg_str) {
            debug!("Indexing: {:?}", msg_dictionary);
            for message_key in message_keys {
                if let Some(handlers) = self.indexers_for_key(message_key) {
                    for handler_id in handlers {
                        if let Some(indexer) = self.indexers.get(*handler_id) {
                            if let Err(e) = indexer.index_dyn(self, events, msg_dictionary, msg_str)
                            {
                                error!(
                                    "Error indexing message:\n{:#?}\n{:#?}\n{:#?}\n{:#?}",
                                    msg_dictionary, e, msg_str, events
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_message_keys(
        &self,
        msg_dictionary: &Value,
        msg_str: &str,
    ) -> Option<Vec<RegistryKey>> {
        let mut keys = vec![];
        for indexer in &self.indexers {
            if let Some(message_key) = indexer.extract_message_key_dyn(msg_dictionary, msg_str) {
                keys.push(message_key);
            }
        }
        if !keys.is_empty() {
            return Some(keys);
        }
        None
    }

    pub fn register_for_key(&mut self, registry_key: &'a str, indexer_id: usize) {
        let key = RegistryKey(registry_key.to_string());
        if let Some(existing_handlers) = self.handlers.get_mut(&key) {
            existing_handlers.push(indexer_id);
        } else {
            let new_handlers = vec![indexer_id];
            self.handlers.insert(key, new_handlers);
        }
    }

    pub fn indexers_for_key(&self, registry_key: &str) -> Option<&Vec<usize>> {
        let registry_key = RegistryKey(registry_key.to_string());
        self.handlers.get(&registry_key)
    }

    pub fn get_indexer(&self, id: usize) -> Option<&dyn IndexerDyn> {
        if let Some(indexer) = self.indexers.get(id) {
            return Some(indexer.as_ref());
        }
        None
    }
}

impl Register for IndexerRegistry {
    fn register(&mut self, indexer: Box<dyn IndexerDyn>, registry_key: Option<&str>) {
        let id = self.indexers.len();
        if let Some(registry_key) = registry_key {
            self.register_for_key(registry_key, id);
        }
        for registry_key in indexer.registry_keys_dyn() {
            debug!("registering {}", &registry_key);
            self.register_for_key(registry_key, id);
        }
        self.indexers.push(indexer);
    }
}

struct TestIndexer {
    pub name: String,
    my_registry_keys: Vec<RegistryKey>,
    my_root_keys: Vec<String>,
}

impl Indexer for TestIndexer {
    type MessageType = ();

    fn id(&self) -> String {
        self.name.clone()
    }

    fn index<'b>(
        &self,
        _registry: &'b IndexerRegistry,
        _events: &'b EventMap,
        _msg_dictionary: &'b Value,
        _msg_str: &'b str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn registry_keys(&self) -> Box<dyn Iterator<Item = &RegistryKey> + '_> {
        Box::from(self.my_registry_keys.iter())
    }

    fn root_keys<'b>(&'b self) -> Box<dyn Iterator<Item = &'b String> + 'b> {
        Box::from(self.my_root_keys.iter())
    }

    fn required_root_keys(&self) -> super::indexer::RootKeysType {
        Box::from([].iter().copied())
    }
}

#[test]
fn test_registry() {
    let indexer_a = TestIndexer {
        name: "indexer_a".to_string(),
        my_registry_keys: vec![
            RegistryKey("key_1".to_string()),
            RegistryKey("key_2".to_string()),
            RegistryKey("key_5".to_string()),
        ],
        my_root_keys: vec![
            "key_1".to_string(),
            "key_2".to_string(),
            "key_5".to_string(),
        ],
    };
    let indexer_b = TestIndexer {
        name: "indexer_b".to_string(),
        my_registry_keys: vec![
            RegistryKey("key_3".to_string()),
            RegistryKey("key_4".to_string()),
        ],
        my_root_keys: vec!["key_3".to_string(), "key_4".to_string()],
    };
    let mut registry = IndexerRegistry::default();
    registry.register(Box::from(indexer_a), None);
    registry.register(Box::from(indexer_b), Some("key_5"));
    let indexer_ids = registry.indexers_for_key("key_5").unwrap();
    assert_eq!(2, indexer_ids.len());
    let should_be_a = registry.get_indexer(indexer_ids[0]).unwrap();
    assert_eq!("indexer_a", should_be_a.id());
}

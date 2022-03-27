use super::event_map::EventMap;
use super::indexer::Indexer;
use diesel::pg::PgConnection;
use serde_json::Value;
use std::collections::HashMap;
use std::slice::Iter;

pub trait Register {
    fn register(&mut self, indexer: Box<dyn Indexer>, registry_key: Option<&str>);
}

pub struct IndexerRegistry {
    pub db: Option<PgConnection>,
    /// Maps string key values to ids of indexers
    handlers: HashMap<String, Vec<usize>>,
    indexers: Vec<Box<dyn Indexer>>,
}

impl Default for IndexerRegistry {
    fn default() -> Self {
        IndexerRegistry::new(None)
    }
}

impl<'a> IndexerRegistry {
    pub fn new(db: Option<PgConnection>) -> Self {
        IndexerRegistry {
            db,
            handlers: HashMap::default(),
            indexers: vec![],
        }
    }

    // This method gets handed the decoded cosmwasm message
    // and asks its registered indexers to index it if they can.
    pub fn index_message_and_events(
        &self,
        events: &EventMap,
        msg_dictionary: &Value,
        msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(message_keys) = &self.extract_message_keys(msg_dictionary, msg_str) {
            println!("Indexing: {:?}", msg_dictionary);
            for message_key in message_keys {
                if let Some(handlers) = self.indexers_for_key(message_key) {
                    for handler_id in handlers {
                        if let Some(indexer) = self.indexers.get(*handler_id) {
                            indexer.index(self, events, msg_dictionary, msg_str)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_message_keys(&self, msg_dictionary: &Value, msg_str: &str) -> Option<Vec<String>> {
        let mut keys = vec![];
        for indexer in &self.indexers {
            if let Some(message_key) = indexer.extract_message_key(msg_dictionary, msg_str) {
                keys.push(message_key);
            }
        }
        if !keys.is_empty() {
            return Some(keys);
        }
        None
    }

    pub fn register_for_key(&mut self, registry_key: &'a str, indexer_id: usize) {
        if let Some(existing_handlers) = self.handlers.get_mut(registry_key) {
            existing_handlers.push(indexer_id);
        } else {
            let new_handlers = vec![indexer_id];
            self.handlers.insert(registry_key.to_string(), new_handlers);
        }
    }

    pub fn indexers_for_key(&self, registry_key: &str) -> Option<&Vec<usize>> {
        self.handlers.get(registry_key)
    }

    pub fn get_indexer(&self, id: usize) -> Option<&dyn Indexer> {
        if let Some(indexer) = self.indexers.get(id) {
            return Some(indexer.as_ref());
        }
        None
    }
}

impl<'a> Register for IndexerRegistry {
    fn register(&mut self, indexer: Box<dyn Indexer>, registry_key: Option<&str>) {
        let id = self.indexers.len();
        if let Some(registry_key) = registry_key {
            self.register_for_key(registry_key, id);
        }
        for registry_key in indexer.registry_keys() {
            println!("registering {}", &registry_key);
            self.register_for_key(registry_key, id);
        }
        self.indexers.push(indexer);
    }
}

struct TestIndexer {
    pub name: String,
    my_registry_keys: Vec<String>,
}

impl<'a> Indexer for TestIndexer {
    fn id(&self) -> String {
        self.name.clone()
    }

    fn index(
        &self,
        _registry: &IndexerRegistry,
        _events: &EventMap,
        _msg_dictionary: &Value,
        _msg_str: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn registry_keys(&self) -> Iter<String> {
        self.my_registry_keys.iter()
    }

    fn extract_message_key(&self, message: &Value, _message_string: &str) -> Option<String> {
        for my_key in &self.my_registry_keys {
            if message.get(my_key).is_some() {
                return Some(my_key.clone());
            }
        }
        None
    }
}

#[test]
fn test_registry() {
    let indexer_a = TestIndexer {
        name: "indexer_a".to_string(),
        my_registry_keys: vec![
            "key_1".to_string(),
            "key_2".to_string(),
            "key_5".to_string(),
        ],
    };
    let indexer_b = TestIndexer {
        name: "indexer_b".to_string(),
        my_registry_keys: vec!["key_3".to_string(), "key_4".to_string()],
    };
    let mut registry = IndexerRegistry::default();
    registry.register(Box::from(indexer_a), None);
    registry.register(Box::from(indexer_b), Some("key_5"));
    let indexer_ids = registry.indexers_for_key("key_5").unwrap();
    assert_eq!(2, indexer_ids.len());
    let should_be_a = registry.get_indexer(indexer_ids[0]).unwrap();
    assert_eq!("indexer_a", should_be_a.id());
}

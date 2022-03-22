use super::wasm_indexer::WasmIndexer;
use std::collections::HashMap;

pub trait Register<'a> {
  fn register(&'a mut self, indexer: Box<dyn WasmIndexer<'a>>, type_url: Option<&'a str>);
}

pub struct WasmIndexerRegistry<'a> {
  handlers: HashMap<String, Vec<Box<dyn WasmIndexer<'a> >>>
}

impl<'a> Register<'a> for WasmIndexerRegistry<'a> {
  fn register(&'a mut self, indexer: Box<dyn WasmIndexer>, type_url: Option<&'a str>) {
    let mut registry_keys = vec!();
    if let Some(type_url) = type_url {
      registry_keys.push(type_url.to_string());
    }

    println!("indexer registry_keys: {:?}", indexer.registry_keys());
    /*
    for registry_key in indexer.registry_keys() {
      registry_keys.push(registry_key.clone());
    }
    println!("registry_keys: {:?}", registry_keys);

    for registry_key in registry_keys {
      let mut indexers;
      let mut existing_indexers = self.handlers.get_mut(&registry_key);
      if let Some(existing_indexers) = existing_indexers {
        indexers = existing_indexers;
      } else {
        let new_indexers: Vec<Box<dyn WasmIndexer<'a> >> = vec![];
        self.handlers.insert(registry_key.clone(), new_indexers);
        indexers = self.handlers.get_mut(&registry_key).unwrap();
      }
      indexers.push(indexer);
    }
    */
  }
}

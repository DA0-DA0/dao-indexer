use diesel::pg::PgConnection;
use serde_json::Value;
use std::slice::Iter;

pub trait Indexer {
  fn index(
    &self,
    db: &PgConnection,
    msg_dictionary: &Value,
    msg_str: &str
  ) -> Result<(), Box<dyn std::error::Error>>;
  fn id(&self) -> String;
  fn registry_keys(&self) -> Iter<String>;
}

use diesel::pg::PgConnection;
use serde_json::Value;

pub trait WasmIndexer<'a> {
  fn index(
    &'a self,
    db: &'a PgConnection,
    msg_dictionary: &'a Value,
    msg_str: &'a str
  ) -> Result<(), Box<dyn std::error::Error>>;

  fn registry_keys(&'a self ) -> &[String];
}

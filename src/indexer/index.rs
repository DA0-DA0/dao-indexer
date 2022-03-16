use diesel::pg::PgConnection;
use std::collections::BTreeMap;

pub trait Index {
  fn index(
    &self,
    db: &PgConnection,
    events: &Option<BTreeMap<String, Vec<String>>>,
  ) -> Result<(), Box<dyn std::error::Error>>; // TODO(gavindoughtie): anyhow::Result<()>
}

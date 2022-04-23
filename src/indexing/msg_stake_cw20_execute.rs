use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use crate::util::debug::dump_events;
use diesel::PgConnection;
use log::debug;
use stake_cw20::msg::ExecuteMsg;

impl IndexMessage for ExecuteMsg {
    fn index_message(
        &self,
        _conn: Option<&PgConnection>,
        _registry: &IndexerRegistry,
        events: &EventMap,
    ) -> anyhow::Result<()> {
        debug!("StakeCw20ExecuteMsg index");
        dump_events(events);
        Ok(())
    }
}

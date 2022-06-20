use clap::Command;
use dao_indexer::config::IndexerConfig;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
// use dao_indexer::indexing::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
// use dao_indexer::indexing::msg_cw3dao_indexer::{
//     Cw3DaoExecuteMsgIndexer, Cw3DaoInstantiateMsgIndexer,
// };
// use dao_indexer::indexing::msg_cw3multisig_indexer::{
//     Cw3MultisigExecuteMsgIndexer, Cw3MultisigInstantiateMsgIndexer,
// };
use dao_indexer::indexing::msg_set::default_msg_set;
// use dao_indexer::indexing::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::schema_indexer::{SchemaRef, SchemaIndexer};
use dao_indexer::indexing::tx::process_tx_info;
use diesel::pg::PgConnection;
use env_logger::Env;
use futures::StreamExt;
use log::{debug, error, info, warn};
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg_026;
use schemars::schema_for;
use sea_orm::{Database, DatabaseConnection};

/// This indexes the Tendermint blockchain starting from a specified block, then
/// listens for new blocks and indexes them with content-aware indexers.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Command::new("Indexer Dao")
    .version("0.0.1")
    .author("Indexer Dao https://daodao.zone/multisig/juno1qertq0ve2mwnpytas6ckwv4d7ny4pqfanjkxanm84dd6g00tl4ssyjk09q")
    .about("CosmWasm Indexer");
    let config = IndexerConfig::with_clap(app);

    let env = Env::default()
        .filter_or("INDEXER_LOG_LEVEL", "info")
        .write_style_or("INDEXER_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    info!("indexing with environment:\n{}", config);

    if !config.postgres_backend {
        warn!("Running indexer without a postgres backend!");
    }

    #[allow(clippy::needless_late_init)]
    let mut registry;
    if config.postgres_backend {
        let diesel_db: PgConnection = establish_connection(&config.database_url);
        let seaql_db: DatabaseConnection = Database::connect(&config.database_url).await?;
        registry = IndexerRegistry::new(Some(diesel_db), Some(seaql_db));
    } else {
        registry = IndexerRegistry::new(None, None);
    }
    // let schema3 = schema_for!(Cw3DaoInstantiateMsg);
    // println!("definitions:\n{:#?}", &schema3.definitions);

    // Register standard indexers:
    // let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    // let cw3dao_instantiate_indexer = Cw3DaoInstantiateMsgIndexer::default();
    // let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    // let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    // let cw3multisig_instantiate_indexer = Cw3MultisigInstantiateMsgIndexer::default();
    // let cw3multisig_execute_indexer = Cw3MultisigExecuteMsgIndexer::default();
    let instantiate_msg_schema = schema_for!(Cw3DaoInstantiateMsg_026);
    let instantiate_msg_label = stringify!(Cw3DaoInstantiateMsg);
    let instantiate_msg_indexer = SchemaIndexer::new(instantiate_msg_label.to_string(), vec![
        SchemaRef {
            name: instantiate_msg_label.to_string(),
            schema: instantiate_msg_schema,
            version: "0.2.6"
        }]);
    registry.register(Box::from(instantiate_msg_indexer), None);
    // registry.register(Box::from(cw20_indexer), None);
    // registry.register(Box::from(cw3multisig_instantiate_indexer), None);
    // registry.register(Box::from(cw3multisig_execute_indexer), None);
    // registry.register(Box::from(cw3dao_instantiate_indexer), None);
    // registry.register(Box::from(cw3dao_indexer), None);
    // registry.register(Box::from(cw20_stake_indexer), None);

    registry.initialize()?;

    if let Some(seaql_db) = &registry.seaql_db {
        // Dump the sql
        println!("Building tables:\n{}", registry.db_builder.sql_string());
        registry.db_builder.create_tables(seaql_db).await?;
    }

    let msg_set = default_msg_set();

    if config.enable_indexer_env {
        let sync_result = block_synchronizer(&registry, &config, msg_set.clone()).await?;
        info!("sync_result:\n{:?}", sync_result);
        if let Ok(msg_set) = msg_set.lock() {
            if !msg_set.unregistered_msgs.is_empty() {
                warn!(
                    "Messages with no handlers:\n{:?}",
                    msg_set.unregistered_msgs
                );
            }
        }
    } else {
        info!("Indexing historical blocks disabled");
    }

    if config.listen {
        let (client, driver) =
            WebSocketClient::new::<&str>(&config.tendermint_websocket_url).await?;
        let driver_handle = tokio::spawn(async move { driver.run().await });

        // Subscribe to transactions (can also add blocks but just Tx for now)
        let mut subs = client.subscribe(EventType::Tx.into()).await?;

        while let Some(res) = subs.next().await {
            let ev = res?;
            let result = ev.data;
            let events = ev.events.unwrap();
            match result {
                EventData::NewBlock { block, .. } => debug!("{:?}", block.unwrap()),
                EventData::Tx { tx_result, .. } => {
                    process_tx_info(&registry, tx_result, &events, msg_set.clone())?
                }
                _ => {
                    error!("Unexpected result {:?}", result)
                }
            }
        }
        // Signal to the driver to terminate.
        match client.close() {
            Ok(val) => info!("closed {:?}", val),
            Err(e) => error!("Error closing client {:?}", e),
        }
        // Await the driver's termination to ensure proper connection closure.
        let _ = driver_handle.await.unwrap();
    }

    Ok(())
}

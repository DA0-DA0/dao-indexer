use dao_indexer::config::IndexerConfig;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::{block_synchronizer, init_known_unknown_messages};
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
use dao_indexer::indexing::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_cw3dao_indexer::Cw3DaoExecuteMsgIndexer;
use dao_indexer::indexing::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::tx::process_tx_info;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use env_logger::Env;
use futures::StreamExt;
use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::sync::Arc;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

/// This indexes the Tendermint blockchain starting from a specified block, then
/// listens for new blocks and indexes them with content-aware indexers.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let config = IndexerConfig::new();

    let env = Env::default()
        .filter_or("INDEXER_LOG_LEVEL", "info")
        .write_style_or("INDEXER_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    info!("indexing with environment:\n{}", config);

    if !config.postgres_backend {
        warn!("Running indexer without a postgres backend!");
    }

    let mut registry;
    if config.postgres_backend {
        let db: PgConnection = establish_connection();
        registry = IndexerRegistry::new(Some(db));
    } else {
        registry = IndexerRegistry::new(None);
    }
    let (client, driver) = WebSocketClient::new::<&str>(&config.tendermint_websocket_url).await?;
    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Register standard indexers:
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    registry.register(Box::from(cw20_indexer), None);
    registry.register(Box::from(cw3dao_indexer), None);
    registry.register(Box::from(cw20_stake_indexer), None);

    let mut msg_set: HashSet<String> = HashSet::new();
    init_known_unknown_messages(&mut msg_set);
    let arc_msg_set = Arc::new(msg_set);

    if config.enable_indexer_env {
        block_synchronizer(&registry, &config, arc_msg_set.clone()).await?;
        warn!("Messages with no handlers:\n{:?}", &arc_msg_set);
    } else {
        info!("Indexing historical blocks disabled");
    }

    if config.listen {
        // Subscribe to transactions (can also add blocks but just Tx for now)
        let mut subs = client.subscribe(EventType::Tx.into()).await?;

        while let Some(res) = subs.next().await {
            let ev = res?;
            let result = ev.data;
            let events = ev.events.unwrap();
            match result {
                EventData::NewBlock { block, .. } => debug!("{:?}", block.unwrap()),
                EventData::Tx { tx_result, .. } => {
                    process_tx_info(&registry, tx_result, &events, arc_msg_set.clone())?
                }
                _ => {
                    error!("Unexpected result {:?}", result)
                }
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

    Ok(())
}

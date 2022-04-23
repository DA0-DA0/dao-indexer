use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
use dao_indexer::indexing::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_cw3dao_indexer::Cw3DaoExecuteMsgIndexer;
use dao_indexer::indexing::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::tx::process_tx_info;
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, error, info, warn};
use parallel_stream::from_stream;
use parallel_stream::ParallelStream;
use std::env;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use std::ops::Deref;

/// This indexes the Tendermint blockchain starting from a specified block, then
/// listens for new blocks and indexes them with content-aware indexers.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let enable_indexer_env = env::var("ENABLE_INDEXER").unwrap_or_else(|_| "false".to_string());
    let tendermint_websocket_url: &str = &env::var("TENDERMINT_WEBSOCKET_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:26657/websocket".to_string());
    let tendermint_rpc_url: &str =
        &env::var("TENDERMINT_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:26657".to_string());
    let tendermint_initial_block = env::var("TENDERMINT_INITIAL_BLOCK_HEIGHT")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u64>()?;
    let tendermint_save_all_blocks = env::var("TENDERMINT_SAVE_ALL_BLOCKS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()?;

    // By default we use a postgres database for the backend, but not always!
    let postgres_backend = env::var("POSTGRES_PERSISTENCE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()?;

    let env = Env::default()
        .filter_or("INDEXER_LOG_LEVEL", "info")
        .write_style_or("INDEXER_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    if !postgres_backend {
        warn!("Running indexer without a postgres backend!");
    }

    let registry = IndexerRegistry::new();

    let pool = establish_connection();
    let (client, driver) = WebSocketClient::new(tendermint_websocket_url).await?;
    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Register standard indexers:
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    registry.register(Box::new(cw20_indexer), None);
    registry.register(Box::new(cw3dao_indexer), None);
    registry.register(Box::new(cw20_stake_indexer), None);

    if enable_indexer_env == "true" {
        let mut db_conn = None;
        if let Some(conn) = pool.try_get() {
            db_conn = Some(conn.deref());
        }
        block_synchronizer(
            &registry,
            db_conn,
            tendermint_rpc_url,
            tendermint_initial_block,
            tendermint_save_all_blocks,
        )
        .await?;
    } else {
        info!("Indexing historical blocks disabled");
    }
    // Subscribe to transactions (can also add blocks but just Tx for now)
    from_stream(client.subscribe(EventType::Tx.into()).await?).for_each(|res| async move {
        let ev = res.unwrap();
        let result = ev.data;
        let events = ev.events.unwrap();
        match result {
            EventData::NewBlock { block, .. } => debug!("{:?}", block.unwrap()),
            EventData::Tx { tx_result, .. } => {
                let mut db_conn = None;
                if let Some(conn) = pool.try_get() {
                    db_conn = Some(conn.deref());
                }              
                process_tx_info(db_conn, &registry, tx_result, &events).unwrap()
            }
            _ => {
                error!("Unexpected result {:?}", result)
            }
        }
    });

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => info!("closed {:?}", val),
        Err(e) => error!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();

    Ok(())
}

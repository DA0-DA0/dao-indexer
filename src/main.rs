pub use cw20::Cw20ExecuteMsg;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
use dao_indexer::indexing::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_cw3dao_indexer::Cw3DaoExecuteMsgIndexer;
use dao_indexer::indexing::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::tx::process_tx_info;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use futures::StreamExt;
use std::env;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let db: PgConnection = establish_connection();
    let (client, driver) = WebSocketClient::new(tendermint_websocket_url).await?;
    let driver_handle = tokio::spawn(async move { driver.run().await });

    let mut registry = IndexerRegistry::new(Some(db));

    // Register standard indexers:
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    registry.register(Box::from(cw20_indexer), None);
    registry.register(Box::from(cw3dao_indexer), None);
    registry.register(Box::from(cw20_stake_indexer), None);

    if enable_indexer_env == "true" {
        block_synchronizer(
            &registry,
            tendermint_rpc_url,
            tendermint_initial_block,
            tendermint_save_all_blocks,
        )
        .await;
    } else {
        println!("Not indexing");
    }
    // Subscribe to transactions (can also add blocks but just Tx for now)
    let mut subs = client.subscribe(EventType::Tx.into()).await?;

    while let Some(res) = subs.next().await {
        let ev = res?;
        let result = ev.data;
        let events = ev.events;
        match result {
            EventData::NewBlock { block, .. } => println!("{:?}", block.unwrap()),
            EventData::Tx { tx_result, .. } => process_tx_info(&registry, tx_result, &events)?,
            _ => eprintln!("unexpected result"),
        }
    }

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => println!("closed {:?}", val),
        Err(e) => eprintln!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();

    Ok(())
}

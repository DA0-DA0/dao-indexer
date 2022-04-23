use bigdecimal::BigDecimal;
pub use cw20::Cw20ExecuteMsg;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::models::NewContract;
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
use dao_indexer::indexing::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_cw3dao_indexer::Cw3DaoExecuteMsgIndexer;
use dao_indexer::indexing::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::tx::process_tx_info;
use diesel::pg::PgConnection;
use diesel::RunQueryDsl;
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, error, info};
use num_bigint::BigInt;
use parallel_stream::from_stream;
use parallel_stream::ParallelStream;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

// TODO(gavin.doughtie): use the anyhow crate

fn test_contract_insert(db: &PgConnection) {
    use dao_indexer::db::schema::contracts::dsl::*;
    let big_u128 = u128::MAX - 10;
    dbg!(big_u128);
    let super_big_int = BigInt::from(big_u128) * BigInt::from(big_u128);
    let myheight = BigDecimal::from(super_big_int.clone());
    let supposed_height = BigInt::from_str(
        "115792089237316195423570985008687907845783772593379917843263342644414228988025",
    )
    .unwrap();
    dbg!(supposed_height == super_big_int);
    dbg!(BigInt::from(big_u128) * BigInt::from(big_u128));
    let contract = NewContract {
        address: "foo",
        staking_contract_address: "bar",
        code_id: -1,
        creator: "gavin",
        admin: "admin_foo",
        label: "label_foo",
        creation_time: "000",
        height: &myheight,
    };
    diesel::insert_into(contracts)
        .values(contract)
        .execute(db)
        .unwrap();
}
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

    let env = Env::default()
        .filter_or("INDEXER_LOG_LEVEL", "info")
        .write_style_or("INDEXER_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    let pool = establish_connection();
    test_contract_insert(&pool.get().unwrap());
    let (client, driver) = WebSocketClient::new(tendermint_websocket_url).await?;
    let driver_handle = tokio::spawn(async move { driver.run().await });

    let mut registry = IndexerRegistry::new(Some(pool));

    // Register standard indexers:
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    registry.register(Box::new(cw20_indexer), None);
    registry.register(Box::new(cw3dao_indexer), None);
    registry.register(Box::new(cw20_stake_indexer), None);

    let registry = Arc::new(registry);

    if enable_indexer_env == "true" {
        block_synchronizer(
            &registry,
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
                process_tx_info(&registry, tx_result, &events).unwrap()
            }
            _ => {
                error!("Unexpected result {:?}", result)
            }
        }
    });

    // while let Some(res) = subs.next().await {
    //     let ev = res?;
    //     let result = ev.data;
    //     let events = ev.events.unwrap();
    //     match result {
    //         EventData::NewBlock { block, .. } => debug!("{:?}", block.unwrap()),
    //         EventData::Tx { tx_result, .. } => process_tx_info(&registry, tx_result, &events)?,
    //         _ => {
    //             error!("Unexpected result {:?}", result)
    //         }
    //     }
    // }

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => info!("closed {:?}", val),
        Err(e) => error!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();

    Ok(())
}

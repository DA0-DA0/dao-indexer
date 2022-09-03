use clap::Command;
pub use cw20::Cw20ExecuteMsg;
use cw3_dao_2_5::msg::ExecuteMsg as Cw3DaoExecuteMsg_025;
use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
use dao_indexer::config::IndexerConfig;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::db_persister::{make_db_ref, DatabasePersister};
use dao_indexer::db::persister::{make_persister_ref, Persister, PersisterRef, StubPersister};
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};
use dao_indexer::indexing::indexers::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::indexers::msg_cw3dao_indexer::{
    Cw3DaoExecuteMsgIndexer, Cw3DaoInstantiateMsgIndexer,
};
use dao_indexer::indexing::indexers::msg_cw3multisig_indexer::{
    Cw3MultisigExecuteMsgIndexer, Cw3MultisigInstantiateMsgIndexer,
};

use cw3_multisig::msg::ExecuteMsg as Cw3MultisigExecuteMsg25;
use cw3_multisig::msg::InstantiateMsg as Cw3MultisigInstantiateMsg25;

use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg25;

use dao_indexer::indexing::indexers::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_set::default_msg_set;
use dao_indexer::indexing::schema_indexer::{SchemaIndexer, SchemaRef};
use dao_indexer::indexing::tx::process_tx_info;
use diesel::pg::PgConnection;
use env_logger::Env;
use futures::StreamExt;
use log::{debug, error, info, warn};
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg_030;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg_030;
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

    let persister_ref: PersisterRef<u64>;

    let mut registry = if config.postgres_backend {
        let diesel_db: PgConnection = establish_connection(&config.database_url);
        let seaql_db: DatabaseConnection = Database::connect(&config.database_url).await?;
        let persister: Box<dyn Persister<Id = u64>> =
            Box::new(DatabasePersister::new(make_db_ref(Box::new(seaql_db))));
        persister_ref = make_persister_ref(persister);
        IndexerRegistry::new(Some(diesel_db), None, persister_ref.clone())
    } else {
        persister_ref = make_persister_ref(Box::new(StubPersister {}));
        // let persister: Box<dyn Persister<Id=u64>> = Box::new(StubPersister {});
        // let persister_ref: PersisterRef<u64> = Arc::new(RwLock::from(RefCell::from(persister)));
        IndexerRegistry::new(None, None, persister_ref.clone())
    };

    // Register standard indexers:
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_instantiate_indexer = Cw3DaoInstantiateMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    let cw3multisig_instantiate_indexer = Cw3MultisigInstantiateMsgIndexer::default();
    let cw3multisig_execute_indexer = Cw3MultisigExecuteMsgIndexer::default();

    // Schema indexer is switched off by default while it's in progress
    if config.schema_indexer {
        // let seaql_db: DatabaseConnection = Database::connect(&config.database_url).await?;

        // TODO(gavindoughtie): I'm *sure* we can make a macro for this
        // pattern, so we can do:
        // register_indexer!(registry, [Cw3DaoInstantiateMsg, Cw3DaoInstantiateMsg25], "0.3.0");
        let msg_label = "Cw3DaoInstantiateMsg";
        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![
                SchemaRef {
                    name: "Cw3DaoInstantiateMsg".to_string(),
                    schema: schema_for!(Cw3DaoInstantiateMsg_030),
                    version: "0.3.0",
                },
                SchemaRef {
                    name: "Cw3DaoInstantiateMsg25".to_string(),
                    schema: schema_for!(Cw3DaoInstantiateMsg25),
                    version: "0.2.5",
                },
            ],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);

        let msg_label = "Cw3DaoExecuteMsg";

        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![
                SchemaRef {
                    name: "Cw3DaoExecuteMsg".to_string(),
                    schema: schema_for!(Cw3DaoExecuteMsg_030),
                    version: "0.3.0",
                },
                SchemaRef {
                    name: "Cw3DaoExecuteMsg25".to_string(),
                    schema: schema_for!(Cw3DaoExecuteMsg_025),
                    version: "0.2.5",
                },
            ],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);

        let msg_label = "Cw20ExecuteMsg";
        // let seaql_db: DatabaseConnection = Database::connect(&config.database_url).await?;

        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![SchemaRef {
                name: "Cw20ExecuteMsg".to_string(),
                schema: schema_for!(Cw20ExecuteMsg),
                version: "0.13.2",
            }],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);

        let msg_label = "Cw3MultisigExecuteMsg";
        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![SchemaRef {
                name: "Cw3MultisigExecuteMsg25".to_string(),
                schema: schema_for!(Cw3MultisigExecuteMsg25),
                version: "0.2.5",
            }],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);

        let msg_label = "Cw3MultisigInstantiateMsg";
        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![SchemaRef {
                name: "Cw3MultisigInstantiateMsg25".to_string(),
                schema: schema_for!(Cw3MultisigInstantiateMsg25),
                version: "0.2.5",
            }],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);

        let msg_label = "StakeCw20ExecuteMsg";
        let msg_indexer = SchemaIndexer::new(
            msg_label.to_string(),
            vec![SchemaRef {
                name: "StakeCw20ExecuteMsg25".to_string(),
                schema: schema_for!(StakeCw20ExecuteMsg25),
                version: "0.2.5",
            }],
            persister_ref.clone(),
        );
        registry.register(Box::from(msg_indexer), None);
    } else {
        registry.register(Box::from(cw20_indexer), None);
        registry.register(Box::from(cw3multisig_instantiate_indexer), None);
        registry.register(Box::from(cw3multisig_execute_indexer), None);
        registry.register(Box::from(cw3dao_instantiate_indexer), None);
        registry.register(Box::from(cw3dao_indexer), None);
        registry.register(Box::from(cw20_stake_indexer), None);
    }
    registry.initialize()?;

    if let Some(seaql_db) = &registry.seaql_db {
        let sql_dump = registry.db_builder.sql_string()?;
        println!("Building tables:\n{}", sql_dump);
        registry.db_builder.create_tables(seaql_db).await?;
    }

    let msg_set = default_msg_set();

    if config.enable_indexer_env {
        block_synchronizer(&registry, &config, msg_set.clone()).await?;
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

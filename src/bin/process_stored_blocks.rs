use clap::Command;
use dao_indexer::db::db_persister::DatabasePersister;
use dao_indexer::db::persister::{make_persister_ref, PersisterRef, StubPersister};
use diesel::PgConnection;
use env_logger::Env;
use log::info;
use sea_orm::{Database, DatabaseConnection};

use dao_indexer::config::IndexerConfig;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::index_search_result;
use dao_indexer::indexing::indexer_registry::{IndexerRegistry, Register};

use dao_indexer::indexing::msg_set::default_msg_set;
use dao_indexer::util::transaction_util::get_transactions;

use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;

use cw20::Cw20ExecuteMsg;
use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
use cw3_multisig::msg::ExecuteMsg as Cw3MultisigExecuteMsg;
use cw3_multisig::msg::InstantiateMsg as Cw3MultisigInstantiateMsg;
use dao_indexer::indexing::schema_indexer::{SchemaIndexer, SchemaRef};
use dao_indexer::{build_and_register_schema_indexer, build_schema_ref};
use schemars::schema_for;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;

fn register_schema_indexers(
    registry: &mut IndexerRegistry,
    persister_ref: PersisterRef<u64>,
) -> anyhow::Result<()> {
    let cw3dao_indexer = SchemaIndexer::<u64>::new(
        "Cw3DaoInstantiateMsg".to_string(),
        vec![
            build_schema_ref!(Cw3DaoInstantiateMsg, "0.2.6"),
            build_schema_ref!(Cw3DaoInstantiateMsg25, "0.2.5"),
        ],
        persister_ref.clone(),
    );
    registry.register(Box::from(cw3dao_indexer), None);

    build_and_register_schema_indexer!(Cw3DaoExecuteMsg, "0.2.6", persister_ref, registry);
    build_and_register_schema_indexer!(Cw20ExecuteMsg, "0.13.4", persister_ref, registry);
    build_and_register_schema_indexer!(Cw3MultisigExecuteMsg, "0.2.5", persister_ref, registry);
    build_and_register_schema_indexer!(Cw3MultisigInstantiateMsg, "0.2.5", persister_ref, registry);
    build_and_register_schema_indexer!(StakeCw20ExecuteMsg, "0.2.4", persister_ref, registry);

    Ok(())
}

async fn init_registry(
    registry: &mut IndexerRegistry,
    persister_ref: PersisterRef<u64>,
) -> anyhow::Result<()> {
    register_schema_indexers(registry, persister_ref.clone())?;
    registry.initialize().await
}

fn process_transactions(config: &IndexerConfig, registry: &IndexerRegistry) -> anyhow::Result<()> {
    let txs = get_transactions(config, registry)?;

    info!("Linearly processing {} transactions \n", txs.len());

    for tx in txs {
        index_search_result(&tx, registry, config, default_msg_set())?;
    }

    Ok(())
}

async fn persist_historical_transactions(
    config: &IndexerConfig,
    diesel_db: PgConnection,
    persister_connection: DatabaseConnection,
    persister_ref: PersisterRef<u64>,
) -> anyhow::Result<()> {
    let mut registry = IndexerRegistry::new(
        Some(diesel_db),
        Some(persister_connection),
        persister_ref.clone(),
    );
    init_registry(&mut registry, persister_ref).await?;
    process_transactions(config, &registry)
}

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

    if config.postgres_backend {
        let diesel_db: PgConnection = establish_connection(&config.database_url);
        let seaql_db: DatabaseConnection = Database::connect(&config.database_url).await?;
        let persister_connection: DatabaseConnection =
            Database::connect(&config.database_url).await?;
        let persister = DatabasePersister::new(seaql_db);
        let persister_ref = make_persister_ref(Box::from(persister));
        persist_historical_transactions(
            &config,
            diesel_db,
            persister_connection,
            persister_ref.clone(),
        )
        .await?;
        drop(persister_ref)
    } else {
        let stub_persister_ref = make_persister_ref(Box::from(StubPersister {}));
        let mut registry = IndexerRegistry::new(None, None, stub_persister_ref.clone());
        init_registry(&mut registry, stub_persister_ref).await?;
        return process_transactions(&config, &registry);
    };
    Ok(())
}

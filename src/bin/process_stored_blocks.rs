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
use dao_indexer::indexing::indexers::msg_cw20_indexer::Cw20ExecuteMsgIndexer;
use dao_indexer::indexing::indexers::msg_cw3dao_indexer::{
    Cw3DaoExecuteMsgIndexer, Cw3DaoInstantiateMsgIndexer,
};
use dao_indexer::indexing::indexers::msg_cw3multisig_indexer::{
    Cw3MultisigExecuteMsgIndexer, Cw3MultisigInstantiateMsgIndexer,
};
use dao_indexer::indexing::indexers::msg_stake_cw20_indexer::StakeCw20ExecuteMsgIndexer;
use dao_indexer::indexing::msg_set::default_msg_set;
use dao_indexer::util::transaction_util::get_transactions;

fn init_registry(registry: &mut IndexerRegistry) -> anyhow::Result<()> {
    let cw20_indexer = Cw20ExecuteMsgIndexer::default();
    let cw3dao_instantiate_indexer = Cw3DaoInstantiateMsgIndexer::default();
    let cw3dao_indexer = Cw3DaoExecuteMsgIndexer::default();
    let cw20_stake_indexer = StakeCw20ExecuteMsgIndexer::default();
    let cw3multisig_instantiate_indexer = Cw3MultisigInstantiateMsgIndexer::default();
    let cw3multisig_execute_indexer = Cw3MultisigExecuteMsgIndexer::default();

    registry.register(Box::from(cw20_indexer), None);
    registry.register(Box::from(cw3multisig_instantiate_indexer), None);
    registry.register(Box::from(cw3multisig_execute_indexer), None);
    registry.register(Box::from(cw3dao_instantiate_indexer), None);
    registry.register(Box::from(cw3dao_indexer), None);
    registry.register(Box::from(cw20_stake_indexer), None);
    registry.initialize()
}

fn process_transactions(config: &IndexerConfig, registry: &IndexerRegistry) -> anyhow::Result<()> {
    let txs = get_transactions(config, registry)?;

    info!("Linearly processing {} transactions \n", txs.len());

    for tx in txs {
        index_search_result(&tx, registry, config, default_msg_set())?;
    }

    Ok(())
}

fn persist_historical_transactions(
    config: &IndexerConfig,
    diesel_db: PgConnection,
    persister_connection: DatabaseConnection,
    persister_ref: PersisterRef<u64>,
) -> anyhow::Result<()> {
    let mut registry =
        IndexerRegistry::new(Some(diesel_db), Some(persister_connection), persister_ref);
    init_registry(&mut registry)?;
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
        )?;
        drop(persister_ref)
    } else {
        let mut registry =
            IndexerRegistry::new(None, None, make_persister_ref(Box::from(StubPersister {})));
        init_registry(&mut registry)?;
        return process_transactions(&config, &registry);
    };
    Ok(())
}

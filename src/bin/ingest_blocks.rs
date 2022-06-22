
// Multi-threading and doing some action. then we can run the multi-threadin
// code there. That is the best way of approaching this solution.

use clap::Command;
use diesel::PgConnection;
use env_logger::Env;

use dao_indexer::config::IndexerConfig;
use dao_indexer::db::connection::establish_connection;
use dao_indexer::historical_parser::block_synchronizer;
use dao_indexer::indexing::indexer_registry::IndexerRegistry;
use dao_indexer::indexing::msg_set::default_msg_set;

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


    let db: PgConnection = establish_connection(&config.database_url);

    let indexer_registry = IndexerRegistry::new(Some(db));
    let msg_set = default_msg_set();


    // println!("testing that this stuff works");

    // Ingestion Engine

    block_synchronizer(&indexer_registry, &config, msg_set.clone()).await?;
    Ok(())
}
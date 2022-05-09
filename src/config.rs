use clap::{Arg, Command};
use dotenv::dotenv;
use std::env;
use std::fmt;

pub struct IndexerConfig {
    pub enable_indexer_env: bool,
    pub tendermint_websocket_url: String,
    pub tendermint_rpc_url: String,
    pub tendermint_initial_block: u64,
    pub tendermint_final_block: u64,
    pub tendermint_save_all_blocks: bool,
    pub postgres_backend: bool,
    pub database_url: String,
    pub transaction_page_size: u8,
    pub block_page_size: u64,
    pub max_requests: u8,
    pub listen: bool,
    pub requeue_sleep: u64,
}

impl IndexerConfig {
    pub fn with_clap(app: Command) -> Self {
        let matches = app
            .arg(
                Arg::new("config")
                    .required(false)
                    .long("config")
                    .takes_value(true)
                    .help("Optionally sets a config file to use"),
            )
            .arg(
                Arg::new("database-url")
                    .required(false)
                    .long("database-url")
                    .takes_value(true)
                    .help("Postgres connection URL"),
            )
            .arg(
                Arg::new("enable-indexer")
                    .required(false)
                    .long("enable-indexer")
                    .takes_value(true)
                    .help("Index historical blocks"),
            )
            /*
            ENABLE_INDEXER=true
            INDEXER_LOG_LEVEL=info
            POSTGRES_PERSISTENCE=false
            BLOCK_PAGE_SIZE=10
            TRANSACTION_PAGE_SIZE=10
             */
            .get_matches();

        let input_file = matches.value_of("config").unwrap_or("");
        if !input_file.is_empty() {
            dotenv::from_filename(input_file).ok();
        } else {
            dotenv().ok();
        }
        Self::init()
    }

    pub fn new() -> Self {
        dotenv().ok();
        Self::init()
    }

    fn init() -> Self {
        let enable_indexer_env = env::var("ENABLE_INDEXER")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let tendermint_websocket_url = env::var("TENDERMINT_WEBSOCKET_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:26657/websocket".to_string());
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://user@localhost:5432/daodaoindexer".to_string());
        let tendermint_rpc_url =
            env::var("TENDERMINT_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:26657".to_string());
        let tendermint_initial_block = env::var("TENDERMINT_INITIAL_BLOCK_HEIGHT")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .unwrap_or(1);
        let tendermint_final_block = env::var("TENDERMINT_FINAL_BLOCK_HEIGHT")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u64>()
            .unwrap_or(0);
        let tendermint_save_all_blocks = env::var("TENDERMINT_SAVE_ALL_BLOCKS")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // By default we use a postgres database for the backend, but not always!
        let postgres_backend = env::var("POSTGRES_PERSISTENCE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let listen = env::var("TENDERMINT_WEBSOCKET_LISTEN")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let transaction_page_size: u8 = env::var("TRANSACTION_PAGE_SIZE")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u8>()
            .unwrap_or(100);

        let block_page_size: u64 = env::var("BLOCK_PAGE_SIZE")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u64>()
            .unwrap_or(100);

        let max_requests: u8 = env::var("MAX_REQUESTS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u8>()
            .unwrap_or(10);

        let requeue_sleep: u64 = env::var("REQUEUE_SLEEP")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u64>()
            .unwrap_or(0);

        IndexerConfig {
            enable_indexer_env,
            tendermint_websocket_url,
            tendermint_rpc_url,
            tendermint_initial_block,
            tendermint_final_block,
            tendermint_save_all_blocks,
            postgres_backend,
            database_url,
            listen,
            transaction_page_size,
            block_page_size,
            max_requests,
            requeue_sleep,
        }
    }
}

impl fmt::Display for IndexerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IndexerConfig: tendermint_rpc_url: {}\n\
        tendermint_websocket_url: {}\n\
        tendermint_initial_block: {}\n\
        tendermint_final_block: {}\n\
        tendermint_save_all_blocks: {}\n\
        postgres_backend: {}\n\
        database_url: {}\n\
        enable_indexer_env: {}\n\
        listen: {}\n\
        transaction_page_size: {}\n\
        block_page_size: {}\n\
        max_requests: {}\n\
        requeue_sleep: {}\n",
            self.tendermint_rpc_url,
            self.tendermint_websocket_url,
            self.tendermint_initial_block,
            self.tendermint_final_block,
            self.tendermint_save_all_blocks,
            self.postgres_backend,
            self.database_url,
            self.enable_indexer_env,
            self.listen,
            self.transaction_page_size,
            self.block_page_size,
            self.max_requests,
            self.requeue_sleep
        )
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self::new()
    }
}

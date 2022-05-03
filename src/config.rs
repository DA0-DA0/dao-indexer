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
    pub transaction_page_size: u8,
    pub block_page_size: u64,
    pub listen: bool,
}

impl IndexerConfig {
    pub fn new() -> Self {
        let enable_indexer_env = env::var("ENABLE_INDEXER")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let tendermint_websocket_url = env::var("TENDERMINT_WEBSOCKET_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:26657/websocket".to_string());
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

        IndexerConfig {
            enable_indexer_env,
            tendermint_websocket_url,
            tendermint_rpc_url,
            tendermint_initial_block,
            tendermint_final_block,
            tendermint_save_all_blocks,
            postgres_backend,
            listen,
            transaction_page_size,
            block_page_size,
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
        enable_indexer_env: {}\n\
        listen: {}\n\
        transaction_page_size: {}\n\
        block_page_size: {}\n",
            self.tendermint_rpc_url,
            self.tendermint_websocket_url,
            self.tendermint_initial_block,
            self.tendermint_final_block,
            self.tendermint_save_all_blocks,
            self.postgres_backend,
            self.enable_indexer_env,
            self.listen,
            self.transaction_page_size,
            self.block_page_size
        )
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self::new()
    }
}

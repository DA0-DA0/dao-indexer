use crate::db::models::{Block, NewBlock};
use crate::indexer::tx::process_parsed;
use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashSet;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;

pub async fn block_synchronizer(
    db: &PgConnection,
    tendermint_rpc_url: &str,
    initial_block_height: u64,
    save_all_blocks: bool,
) {
    use crate::db::schema::block::dsl::*;
    let tendermint_client = TendermintClient::new(tendermint_rpc_url).unwrap();

    let latest_block_response = tendermint_client.latest_block_results().await.unwrap();
    let latest_block_height = latest_block_response.height.value();

    let blocks_from_db = block.load::<Block>(db).unwrap();

    let mut set_of_already_indexed_blocks = HashSet::new();

    for db_block in blocks_from_db {
        set_of_already_indexed_blocks.insert(db_block.height);
    }
    for block_height in initial_block_height..latest_block_height {
        if !set_of_already_indexed_blocks.contains(&(block_height as i64)) {
            if block_height % 1000 == 0 {
                println!("Added another 1000 blocks, height: {}", block_height);
            }

            let response = tendermint_client.block(block_height as u32).await.unwrap();
            let block_hash = response.block_id.hash.to_string();
            if save_all_blocks {
                let new_block = NewBlock::from_block_response(&block_hash, &response.block);

                diesel::insert_into(block)
                    .values(&new_block)
                    .execute(db)
                    .expect("Error saving new Block");
            }
            for tx in response.block.data.iter() {
                let unmarshalled_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
                let _ = process_parsed(db, &unmarshalled_tx, &None);
                //process_tx_info(db, &unmarshalled_tx.tx_info, events);
                for tx_message in unmarshalled_tx.body.messages {
                    // TODO(jamesortega): Attach here gavins code to index based on the type of transaction
                    classify_transaction(tx_message)
                }
            }
        }
    }
}

fn classify_transaction(tx: cosmrs::Any) {
    match tx.type_url.to_string().as_str() {
        "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
            println!("we found an instnatiate contract, p0g")
        }
        _ => {
            println!("No handler for {}", tx.type_url.to_string().as_str());
        }
    }
}

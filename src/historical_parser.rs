use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tendermint_rpc::Client;
use tendermint_rpc::{HttpClient as TendermintClient};
use crate::db::models::{NewBlock, Block};
use crate::db::schema::block::dsl::*;
use std::collections::HashSet;

pub async fn block_synchronizer(db: &PgConnection) {
    // TODO(gavindoughtie): Get URL from env
    let tendermint_client = TendermintClient::new("http://127.0.0.1:26657").unwrap();

    let latest_block_response = tendermint_client.latest_block_results().await.unwrap();
    let latest_block_height = latest_block_response.height.value();

    let blocks_from_db = block.load::<Block>(db).unwrap();

    let mut set_of_already_indexed_blocks = HashSet::new();

    for db_block in blocks_from_db {
        set_of_already_indexed_blocks.insert(db_block.height);
    }
    
    for block_height in 1..latest_block_height {
        if !set_of_already_indexed_blocks.contains(&(block_height as i64)) {
            println!("Adding block: {}", block_height);

            let response = tendermint_client.block(block_height as u32).await.unwrap();
            let block_hash = response.block_id.hash.to_string();
            let new_block = NewBlock::from_block_response(&block_hash, &response.block);
    
            diesel::insert_into(block)
                .values(&new_block)
                .execute(db)
                .expect("Error saving new Block");
    
            for tx in response.block.data.iter() {
                let unmarshalled_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
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
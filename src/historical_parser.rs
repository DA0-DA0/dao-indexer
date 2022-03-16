use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tendermint_rpc::Client;
use tendermint_rpc::{HttpClient as TendermintClient};
use crate::db::models::{NewBlock, Block};
use crate::db::schema::block::dsl::*;

pub async fn block_synchronizer(db: &PgConnection) {
    // TODO(gavindoughtie): Get URL from env
    let tendermint_client = TendermintClient::new("http://127.0.0.1:26657").unwrap();

    let latest_block_response = tendermint_client.latest_block_results().await.unwrap();
    let latest_block_height = latest_block_response.height.value();
    
    for block_height in 1..latest_block_height {
        
        let db_block_opt: Option<Block> = block
            .find(block_height as i64)
            .get_result::<Block>(db).optional().unwrap();

        if let Some(db_block) = db_block_opt {
            println!("Already stored block at height: {}", db_block.height);
        } else {
            println!("Indexing block at block height: {}", block_height);
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
                    // TODO(entrancedjames): Attach here gavins code to index based on the type of transaction
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
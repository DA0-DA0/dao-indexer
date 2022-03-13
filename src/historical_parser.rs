use crate::db::models::NewBlock;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use tendermint_rpc::{HttpClient as TendermintClient};
use crate::db::schema::block::dsl::*;
// use dao_indexer_rs::db::schema::block::dsl::*;


// going to need async, the main things we'll need are the link to the database and 
pub async fn blocker(db: &PgConnection) {
    // Instantiate tendermint client
    // begin reading database for completed chunks, and find the missing chunks

    let tendermint_client = TendermintClient::new("http://127.0.0.1:26657").unwrap();
    for block_height in 1..10 {
        let response = tendermint_client.block( block_height as u32).await.unwrap();
        println!("{}", response.block_id.hash);

        let new_block = NewBlock {
            height: response.block.header.height.value() as i64,
            hash: &response.block_id.hash.to_string(),
            num_txs: response.block.data.iter().len() as i64,
        };

        diesel::insert_into(block)
            .values(&new_block)
            .execute(&db)
            .expect("Error saving new Block");
    }

}
